use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use workdesk_core::{AppConfig, AppUpdateFeed, AppUpdateManifest};
use workdesk_desktop::app_updater::DesktopAppUpdater;
use workdesk_desktop::runtime_bootstrap::RuntimeBootstrapper;

fn restore_var(key: &str, value: Option<String>) {
    if let Some(value) = value {
        std::env::set_var(key, value);
    } else {
        std::env::remove_var(key);
    }
}

fn signed_manifest(
    channel: &str,
    package_url: &str,
    package: &[u8],
) -> (AppUpdateManifest, String) {
    let signing_key = SigningKey::from_bytes(&[11u8; 32]);
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());
    let unsigned = AppUpdateManifest {
        channel: channel.into(),
        version: "1.2.3".into(),
        package_url: package_url.into(),
        package_sha256: format!("{:x}", Sha256::digest(package)),
        signature: String::new(),
    };
    let signature = STANDARD.encode(
        signing_key
            .sign(unsigned.signing_payload().as_bytes())
            .to_bytes(),
    );
    (
        AppUpdateManifest {
            signature,
            ..unsigned
        },
        public_key,
    )
}

#[tokio::test]
async fn runtime_bootstrap_seeds_sidecar_and_onlyoffice_from_install_resources() {
    let tmp = TempDir::new().expect("tempdir");
    let install_root = tmp.path().join("install");
    let local_appdata = tmp.path().join("local");
    let workspace_root = tmp.path().join("workspace");
    let db_path = local_appdata
        .join("WorkDeskStudio")
        .join("data")
        .join("workdesk.db");
    let sidecar_bundle = install_root.join("resources").join("sidecar");
    let onlyoffice_bundle = install_root.join("resources").join("onlyoffice");

    tokio::fs::create_dir_all(sidecar_bundle.join("node"))
        .await
        .expect("create sidecar bundle");
    tokio::fs::create_dir_all(&onlyoffice_bundle)
        .await
        .expect("create onlyoffice bundle");
    tokio::fs::write(sidecar_bundle.join("node").join("node.exe"), "node")
        .await
        .expect("write node bundle");
    tokio::fs::write(sidecar_bundle.join("sidecar.js"), "console.log('sidecar');")
        .await
        .expect("write sidecar script");
    tokio::fs::write(onlyoffice_bundle.join("documentserver.exe"), "docserver")
        .await
        .expect("write onlyoffice binary");

    let old_localappdata = std::env::var("LOCALAPPDATA").ok();
    let old_install_root = std::env::var("WORKDESK_INSTALL_ROOT").ok();
    let old_workspace = std::env::var("WORKDESK_WORKSPACE_ROOT").ok();
    let old_db_path = std::env::var("WORKDESK_DB_PATH").ok();
    std::env::set_var("LOCALAPPDATA", &local_appdata);
    std::env::set_var("WORKDESK_INSTALL_ROOT", &install_root);
    std::env::set_var("WORKDESK_WORKSPACE_ROOT", &workspace_root);
    std::env::set_var("WORKDESK_DB_PATH", &db_path);

    let config = AppConfig::from_env().expect("app config");
    let bootstrapper = RuntimeBootstrapper::new(config.clone());
    let seeded = bootstrapper.ensure_seeded().await.expect("seed runtimes");
    assert!(seeded.seeded_sidecar);
    assert!(seeded.seeded_onlyoffice);
    assert!(config.sidecar_path.exists());
    assert!(config.sidecar_script_path.exists());
    assert!(config.onlyoffice_binary_path.exists());

    let second = bootstrapper
        .ensure_seeded()
        .await
        .expect("seed runtimes again");
    assert!(!second.seeded_sidecar);
    assert!(!second.seeded_onlyoffice);

    restore_var("LOCALAPPDATA", old_localappdata);
    restore_var("WORKDESK_INSTALL_ROOT", old_install_root);
    restore_var("WORKDESK_WORKSPACE_ROOT", old_workspace);
    restore_var("WORKDESK_DB_PATH", old_db_path);
}

#[tokio::test]
async fn desktop_updater_downloads_and_verifies_selected_channel() {
    let tmp = TempDir::new().expect("tempdir");
    let package = b"msi-binary";
    let package_path = tmp.path().join("WorkDeskStudio-1.2.3.msi");
    tokio::fs::write(&package_path, package)
        .await
        .expect("write package");
    let (manifest, public_key) =
        signed_manifest("stable", &package_path.display().to_string(), package);
    let feed = AppUpdateFeed {
        manifests: vec![manifest],
    };
    let feed_path = tmp.path().join("feed.json");
    tokio::fs::write(
        &feed_path,
        serde_json::to_vec(&feed).expect("serialize feed"),
    )
    .await
    .expect("write feed");

    let updater = DesktopAppUpdater::new(
        format!("file://{}", feed_path.display()),
        public_key,
        tmp.path().join("downloads"),
    );
    let prepared = updater
        .prepare_update("stable")
        .await
        .expect("prepare update");

    assert_eq!(prepared.version, "1.2.3");
    assert!(prepared.installer_path.exists());
    let bytes = tokio::fs::read(prepared.installer_path)
        .await
        .expect("read prepared installer");
    assert_eq!(bytes, package);
}
