use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use workdesk_core::{build_router, CoreRepository, CoreService, SqliteCoreRepository};

async fn setup_router() -> axum::Router {
    let root = std::env::temp_dir().join(format!("workdesk-api-test-{}", Uuid::new_v4()));
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

#[tokio::test]
async fn health_returns_success_envelope() {
    let app = setup_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
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

    assert!(payload.get("data").is_some());
    assert!(payload["error"].is_null());
    assert!(payload["meta"]["request_id"].is_string());
    assert!(payload["meta"]["timestamp"].is_string());
}

#[tokio::test]
async fn not_found_returns_error_envelope() {
    let app = setup_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/workflows/not-found")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), 404);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");

    assert!(payload["data"].is_null());
    assert_eq!(payload["error"]["code"], "WORKFLOW_NOT_FOUND");
    assert!(payload["error"]["message"].is_string());
    assert!(payload["meta"]["request_id"].is_string());
}
