use workdesk_domain::{
    ApprovalState, CodexIpcMeta, CodexIpcRequest, CodexIpcResponse, WorkflowChangeProposal,
    WorkflowDefinition, WorkflowEdge, WorkflowNode, WorkflowNodeKind, WorkflowStatus,
};

#[test]
fn workflow_definition_rejects_cyclic_graphs() {
    let workflow = WorkflowDefinition {
        id: "wf-1".into(),
        name: "Cycle".into(),
        timezone: "Asia/Taipei".into(),
        nodes: vec![
            WorkflowNode::new("a", WorkflowNodeKind::ScheduleTrigger),
            WorkflowNode::new("b", WorkflowNodeKind::AgentPrompt),
        ],
        edges: vec![WorkflowEdge::new("a", "b"), WorkflowEdge::new("b", "a")],
        version: 1,
        status: WorkflowStatus::Draft,
        agent_defaults: None,
    };

    let err = workflow
        .validate()
        .expect_err("expected cycle validation error");
    assert!(err.to_string().contains("cycle"));
}

#[test]
fn workflow_proposal_requires_pending_before_approval() {
    let mut proposal =
        WorkflowChangeProposal::new("wf-1".into(), "update diff".into(), "agent".into());
    proposal.approval_state = ApprovalState::Applied;
    let err = proposal
        .approve("admin".into())
        .expect_err("cannot approve already applied proposal");
    assert!(err.to_string().contains("pending"));
}

#[test]
fn codex_ipc_contract_roundtrip_json() {
    let request = CodexIpcRequest {
        request_type: "run_prompt".into(),
        payload: serde_json::json!({
            "session_id": "session-1",
            "prompt": "hello"
        }),
        request_id: "req-1".into(),
    };
    let encoded = serde_json::to_string(&request).expect("encode request");
    let decoded: CodexIpcRequest = serde_json::from_str(&encoded).expect("decode request");
    assert_eq!(decoded.request_type, "run_prompt");
    assert_eq!(decoded.request_id, "req-1");

    let response = CodexIpcResponse {
        ok: true,
        data: Some(serde_json::json!({"output":"ok"})),
        error: None,
        meta: CodexIpcMeta {
            request_id: "req-1".into(),
            timestamp: "2026-03-06T00:00:00Z".into(),
        },
    };
    let response_json = serde_json::to_string(&response).expect("encode response");
    let parsed: CodexIpcResponse = serde_json::from_str(&response_json).expect("decode response");
    assert!(parsed.ok);
    assert_eq!(parsed.meta.request_id, "req-1");
}
