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
async fn workflow_patch_endpoint_updates_definition_with_envelope() {
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
                      "name":"canvas",
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

    let patch_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/workflows/{workflow_id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "name":"canvas-v2",
                      "agent_defaults":{
                        "model":"gpt-5.4",
                        "model_reasoning_effort":"high"
                      },
                      "nodes":[
                        {
                          "id":"n1",
                          "kind":"schedule_trigger",
                          "x":320.5,
                          "y":140.25,
                          "config":{"cron":"0 * * * *"}
                        }
                      ],
                      "edges":[]
                    }"#,
                ))
                .expect("patch request"),
        )
        .await
        .expect("patch response");
    assert_eq!(patch_response.status(), 200);
    let patch_body = to_bytes(patch_response.into_body(), usize::MAX)
        .await
        .expect("patch body");
    let patch_payload: Value = serde_json::from_slice(&patch_body).expect("patch json");
    assert!(patch_payload["error"].is_null());
    assert_eq!(patch_payload["data"]["name"], "canvas-v2");
    assert_eq!(patch_payload["data"]["version"], 2);
    assert_eq!(patch_payload["data"]["nodes"][0]["x"], 320.5);
    assert_eq!(
        patch_payload["data"]["agent_defaults"]["model_reasoning_effort"],
        "high"
    );

    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/workflows/{workflow_id}"))
                .body(Body::empty())
                .expect("get request"),
        )
        .await
        .expect("get response");
    assert_eq!(get_response.status(), 200);
    let get_body = to_bytes(get_response.into_body(), usize::MAX)
        .await
        .expect("get body");
    let get_payload: Value = serde_json::from_slice(&get_body).expect("get json");
    assert_eq!(get_payload["data"]["name"], "canvas-v2");
    assert_eq!(get_payload["data"]["nodes"][0]["y"], 140.25);
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

#[tokio::test]
async fn workbench_session_routes_use_envelope() {
    let app = setup_router().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agent/sessions")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "title": "Workbench",
                      "config": {
                        "model": "gpt-5.4",
                        "model_reasoning_effort": "high",
                        "speed": true,
                        "plan_mode": true
                      },
                      "last_active_panel": "runs"
                    }"#,
                ))
                .expect("create request"),
        )
        .await
        .expect("create response");
    assert_eq!(create_response.status(), 200);

    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("create body");
    let create_payload: Value = serde_json::from_slice(&create_body).expect("create json");
    assert!(create_payload["error"].is_null());
    let session_id = create_payload["data"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string();

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agent/sessions")
                .body(Body::empty())
                .expect("list request"),
        )
        .await
        .expect("list response");
    assert_eq!(list_response.status(), 200);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .expect("list body");
    let list_payload: Value = serde_json::from_slice(&list_body).expect("list json");
    assert_eq!(list_payload["data"][0]["session_id"], session_id);

    let prompt_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/agent/sessions/{session_id}/choice-prompts"
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{
                      "question": "Choose deployment path",
                      "options": [
                        {
                          "option_id": "safe",
                          "label": "Safe",
                          "description": "Lower risk"
                        },
                        {
                          "option_id": "fast",
                          "label": "Fast",
                          "description": "Faster shipping"
                        }
                      ],
                      "recommended_option_id": "safe",
                      "allow_freeform": true
                    }"#,
                ))
                .expect("prompt request"),
        )
        .await
        .expect("prompt response");
    assert_eq!(prompt_response.status(), 200);

    let prompt_body = to_bytes(prompt_response.into_body(), usize::MAX)
        .await
        .expect("prompt body");
    let prompt_payload: Value = serde_json::from_slice(&prompt_body).expect("prompt json");
    assert_eq!(prompt_payload["data"]["recommended_option_id"], "safe");
    assert!(prompt_payload["error"].is_null());
}
