use workdesk_core::{
    ApprovalState, CreateWorkflowInput, InMemoryCoreService, WorkflowNodeInput, WorkflowNodeKind,
};

#[tokio::test]
async fn proposal_must_be_pending_to_approve() {
    let service = InMemoryCoreService::default();
    let wf = service
        .create_workflow(CreateWorkflowInput {
            name: "ops".into(),
            timezone: "Asia/Taipei".into(),
            nodes: vec![WorkflowNodeInput {
                id: "n1".into(),
                kind: WorkflowNodeKind::ScheduleTrigger,
            }],
            edges: vec![],
        })
        .await
        .expect("create workflow");

    let mut proposal = service
        .propose_workflow_change(wf.id.clone(), "diff".into(), "agent".into())
        .await
        .expect("create proposal");
    proposal.approval_state = ApprovalState::Applied;

    let err = service
        .approve_workflow_change_with_state(proposal)
        .await
        .expect_err("should reject non-pending proposal");
    assert!(err.to_string().contains("pending"));
}
