use workdesk_domain::{
    ApprovalState, WorkflowChangeProposal, WorkflowDefinition, WorkflowEdge, WorkflowNode,
    WorkflowNodeKind, WorkflowStatus,
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
