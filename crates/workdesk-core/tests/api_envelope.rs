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

#[tokio::test]
async fn workflow_run_endpoint_enqueues_run_with_envelope() {
    let app = setup_router().await;

    let create_workflow = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/workflows")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "name":"ops",
                      "timezone":"Asia/Taipei",
                      "nodes":[{"id":"n1","kind":"schedule_trigger"}],
                      "edges":[]
                    }"#,
                ))
                .expect("create workflow request"),
        )
        .await
        .expect("create workflow response");
    assert_eq!(create_workflow.status(), 200);

    let workflow_body = to_bytes(create_workflow.into_body(), usize::MAX)
        .await
        .expect("workflow body");
    let workflow_payload: Value = serde_json::from_slice(&workflow_body).expect("workflow json");
    let workflow_id = workflow_payload["data"]["id"]
        .as_str()
        .expect("workflow id")
        .to_string();

    let run_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/workflows/{workflow_id}/run"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"requested_by":"tester"}"#))
                .expect("run request"),
        )
        .await
        .expect("run response");
    assert_eq!(run_response.status(), 200);

    let run_body = to_bytes(run_response.into_body(), usize::MAX)
        .await
        .expect("run body");
    let run_payload: Value = serde_json::from_slice(&run_body).expect("run json");
    assert!(run_payload["error"].is_null());
    assert_eq!(run_payload["data"]["workflow_id"], workflow_id);
    assert_eq!(run_payload["data"]["status"], "queued");

    let run_id = run_payload["data"]["run_id"]
        .as_str()
        .expect("run id")
        .to_string();
    let nodes_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runs/{run_id}/nodes"))
                .body(Body::empty())
                .expect("nodes request"),
        )
        .await
        .expect("nodes response");
    assert_eq!(nodes_response.status(), 200);

    let nodes_body = to_bytes(nodes_response.into_body(), usize::MAX)
        .await
        .expect("nodes body");
    let nodes_payload: Value = serde_json::from_slice(&nodes_body).expect("nodes json");
    let nodes = nodes_payload["data"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["node_id"], "n1");
    assert_eq!(nodes[0]["status"], "pending");
}

#[tokio::test]
async fn onlyoffice_callback_uses_envelope() {
    let app = setup_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/office/onlyoffice/callback")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "payload": {
                        "status": 2,
                        "url": "https://example.com/file.docx"
                      }
                    }"#,
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
    assert_eq!(payload["data"]["accepted"], true);
    assert!(payload["error"].is_null());
}
