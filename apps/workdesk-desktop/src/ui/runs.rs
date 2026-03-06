use super::*;
use workdesk_core::{RunStatus, WorkflowRunEvent};

pub(super) fn render_runs(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let selected_run = snapshot.selected_run_id.clone();
    let controller = view.controller.clone();
    let runtime = view.runtime.clone();

    let list = snapshot.runs.iter().enumerate().fold(
        div().flex().flex_col().gap_1(),
        |list, (index, run)| {
            let run_id = run.run_id.clone();
            let item_controller = controller.clone();
            let item_runtime = runtime.clone();
            let mut button = Button::new(("run", index))
                .label(format!(
                    "{} [{}] {}",
                    run.run_id,
                    format_run_status(&run.status),
                    format_run_time(run)
                ))
                .on_click(move |_, _, _| {
                    let controller = item_controller.clone();
                    let run_id = run_id.clone();
                    item_runtime.spawn(async move {
                        let _ = controller
                            .dispatch_command(crate::command::DesktopCommand::OpenRun { run_id })
                            .await;
                    });
                });
            if selected_run.as_deref() == Some(run.run_id.as_str()) {
                button = button.primary();
            }
            list.child(button)
        },
    );

    div()
        .flex()
        .size_full()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_1_2()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("cancel-run")
                                .label("Cancel")
                                .on_click(cx.listener(WorkdeskMainView::on_cancel_run)),
                        )
                        .child(
                            Button::new("retry-run")
                                .label("Retry")
                                .on_click(cx.listener(WorkdeskMainView::on_retry_run)),
                        ),
                )
                .child("Run List")
                .child(list),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .w_1_2()
                .child(format!(
                    "Run Detail: {}",
                    snapshot
                        .selected_run_id
                        .clone()
                        .unwrap_or_else(|| "(none)".into())
                ))
                .child(render_event_list(&snapshot.run_events))
                .child(render_node_list(&snapshot.run_nodes))
                .child(render_skill_list(&snapshot.run_skills)),
        )
}

fn format_run_status(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Canceled => "canceled",
    }
}

fn format_run_time(run: &workdesk_core::WorkflowRun) -> String {
    if let Some(finished) = run.finished_at {
        return finished.to_rfc3339();
    }
    if let Some(started) = run.started_at {
        return started.to_rfc3339();
    }
    run.created_at.to_rfc3339()
}

fn render_event_list(events: &[WorkflowRunEvent]) -> impl IntoElement {
    events.iter().take(50).fold(
        div().flex().flex_col().gap_1().child("Events"),
        |div, event| {
            div.child(format!(
                "#{} {} {}",
                event.seq, event.event_type, event.payload
            ))
        },
    )
}

fn render_skill_list(skills: &[workdesk_core::RunSkillSnapshot]) -> impl IntoElement {
    skills.iter().take(50).fold(
        div().flex().flex_col().gap_1().child("Skills Snapshot"),
        |div, skill| {
            div.child(format!(
                "{} ({:?}) v{} => {}",
                skill.name,
                skill.scope,
                skill.version,
                skill
                    .materialized_path
                    .clone()
                    .unwrap_or_else(|| skill.content_path.clone())
            ))
        },
    )
}

fn render_node_list(nodes: &[workdesk_core::WorkflowRunNodeState]) -> impl IntoElement {
    nodes.iter().take(50).fold(
        div().flex().flex_col().gap_1().child("Run Nodes"),
        |div, node| {
            div.child(format!(
                "{} {:?} attempt={} {:?}",
                node.node_id, node.kind, node.attempt, node.status
            ))
        },
    )
}
