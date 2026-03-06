use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use workdesk_core::{AppUpdateFeed, AppUpdateManifest};

fn signed_manifest(channel: &str, package: &[u8]) -> (AppUpdateManifest, String) {
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());
    let unsigned = AppUpdateManifest {
        channel: channel.into(),
        version: "1.0.1".into(),
        package_url: "https://updates.example.com/WorkDeskStudio-1.0.1.msi".into(),
        package_sha256: format!("{:x}", Sha256::digest(package)),
        signature: String::new(),
    };
    let signature = STANDARD.encode(signing_key.sign(unsigned.signing_payload().as_bytes()).to_bytes());
    let manifest = AppUpdateManifest {
        signature,
        ..unsigned
    };
    (manifest, public_key)
}

#[test]
fn update_feed_selects_channel_and_verifies_signature_and_package() {
    let package = b"workdesk-msi";
    let (stable_manifest, public_key) = signed_manifest("stable", package);
    let feed = AppUpdateFeed {
        manifests: vec![stable_manifest.clone()],
    };

    let selected = feed.select_channel("stable").expect("stable channel");
    assert_eq!(selected.version, "1.0.1");
    selected
        .verify_signature(&public_key)
        .expect("signature should verify");
    selected
        .verify_package(package, &public_key)
        .expect("package should verify");
}

#[test]
fn update_verifier_rejects_package_hash_mismatch() {
    let package = b"workdesk-msi";
    let (manifest, public_key) = signed_manifest("beta", package);

    let error = manifest
        .verify_package(b"tampered-package", &public_key)
        .expect_err("tampered package should fail");
    assert!(error.to_string().contains("sha256"));
}

#[test]
fn update_feed_rejects_unknown_channel() {
    let package = b"workdesk-msi";
    let (manifest, _) = signed_manifest("stable", package);
    let feed = AppUpdateFeed {
        manifests: vec![manifest],
    };

    let error = feed
        .select_channel("canary")
        .expect_err("unknown channel should fail");
    assert!(error.to_string().contains("channel"));
}

#[tokio::test]
async fn update_feed_loads_from_http_source() {
    let package = b"workdesk-msi";
    let (manifest, _) = signed_manifest("stable", package);
    let feed = AppUpdateFeed {
        manifests: vec![manifest],
    };
    let body = serde_json::to_string(&feed).expect("serialize feed");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = [0u8; 1024];
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = socket.read(&mut request).await.expect("read request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    });

    let loaded = AppUpdateFeed::load(&format!("http://{addr}/feed.json"))
        .await
        .expect("load update feed");
    server.await.expect("server task");

    assert_eq!(loaded, feed);
}
