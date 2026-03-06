use axum::body::{to_bytes, Body};
use axum::http::Request;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use workdesk_core::{build_router, CoreRepository, CoreService, SqliteCoreRepository};

async fn setup_router() -> axum::Router {
    let root = std::env::temp_dir().join(format!("workdesk-fs-test-{}", Uuid::new_v4()));
    let db_path = root.join("workdesk.db");
    let workspace_root = root.join("workspace");
    tokio::fs::create_dir_all(&workspace_root)
        .await
        .expect("create workspace root");

    let repo = SqliteCoreRepository::connect(&db_path)
        .await
        .expect("connect sqlite");
    repo.migrate().await.expect("run migrations");
    let service = CoreService::new(Arc::new(repo), workspace_root);
    build_router(service)
}

async fn write_file(app: &axum::Router, path: &str, content: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/fs/file")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "path": path,
                        "content_base64": STANDARD.encode(content.as_bytes())
                    })
                    .to_string(),
                ))
                .expect("write request"),
        )
        .await
        .expect("write response");
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn fs_search_returns_matches() {
    let app = setup_router().await;
    write_file(&app, "notes/a.txt", "alpha hello world").await;
    write_file(&app, "notes/b.txt", "beta goodbye").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/fs/search?path=notes&query=hello&limit=20")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), 200);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");
    let data = payload["data"].as_array().expect("data array");
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["path"], "notes/a.txt");
}

#[tokio::test]
async fn fs_diff_returns_hunks() {
    let app = setup_router().await;
    write_file(&app, "notes/left.txt", "line1\nline2\nline3\n").await;
    write_file(&app, "notes/right.txt", "line1\nlineX\nline3\n").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/fs/diff")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "left_path": "notes/left.txt",
                        "right_path": "notes/right.txt"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), 200);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");
    let hunks = payload["data"]["hunks"].as_array().expect("hunks");
    assert!(hunks.iter().any(|h| h["kind"] == "delete"));
    assert!(hunks.iter().any(|h| h["kind"] == "insert"));
}

#[tokio::test]
async fn terminal_session_start_and_get_output() {
    let app = setup_router().await;
    write_file(&app, "notes/readme.txt", "terminal").await;

    let start_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/fs/terminal/start")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "path": "notes",
                        "command": if cfg!(windows) { "cmd /C echo hello_terminal" } else { "sh -lc 'echo hello_terminal'" }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(start_response.status(), 200);

    let start_bytes = to_bytes(start_response.into_body(), usize::MAX)
        .await
        .expect("read start body");
    let start_payload: Value = serde_json::from_slice(&start_bytes).expect("start json");
    let session_id = start_payload["data"]["session_id"]
        .as_str()
        .expect("session id");

    let output_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/fs/terminal/session/{session_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(output_response.status(), 200);

    let output_bytes = to_bytes(output_response.into_body(), usize::MAX)
        .await
        .expect("read output body");
    let output_payload: Value = serde_json::from_slice(&output_bytes).expect("output json");
    let stdout = output_payload["data"]["stdout"]
        .as_str()
        .expect("stdout text");
    assert!(
        stdout.contains("hello_terminal"),
        "stdout did not contain expected marker: {stdout}"
    );
}
