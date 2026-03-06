use crate::command::DesktopCommand;
use crate::controller::{DesktopAppController, UiRoute};
use anyhow::Result;
use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, IntoElement, Render, Timer,
    Window, WindowBounds, WindowOptions,
};
use gpui_component::{button::Button, button::ButtonVariants, Root};
use std::time::Duration;
use workdesk_core::{RunSkillSnapshot, RunStatus, WorkflowRunEvent};

pub fn run_gpui(
    controller: DesktopAppController,
    locale: String,
    runtime: tokio::runtime::Handle,
) -> Result<()> {
    Application::new().run(move |cx: &mut App| {
        gpui_component::set_locale(&locale);
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1180.0), px(780.0)), cx);
        let controller_for_window = controller.clone();
        let runtime_for_window = runtime.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| {
                    WorkdeskMainView::new(
                        controller_for_window.clone(),
                        runtime_for_window.clone(),
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open WorkDesk Studio window");
        cx.activate(true);
    });
    Ok(())
}

struct WorkdeskMainView {
    controller: DesktopAppController,
    runtime: tokio::runtime::Handle,
    last_revision: u64,
    last_focus_seq: u64,
}

impl WorkdeskMainView {
    fn new(
        controller: DesktopAppController,
        runtime: tokio::runtime::Handle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |entity, cx| loop {
            Timer::after(Duration::from_millis(450)).await;
            if entity
                .update(cx, |_, cx| {
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        })
        .detach();

        Self {
            controller,
            runtime,
            last_revision: 0,
            last_focus_seq: 0,
        }
    }

    fn on_refresh(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.refresh_runs().await;
            let _ = controller.refresh_selected_run_detail().await;
        });
    }

    fn on_cancel_run(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.cancel_selected_run().await;
        });
    }

    fn on_retry_run(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.retry_selected_run().await;
        });
    }
}

impl Render for WorkdeskMainView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.controller.snapshot();
        if snapshot.focus_seq > self.last_focus_seq {
            self.last_focus_seq = snapshot.focus_seq;
            window.activate_window();
            cx.activate(true);
        }
        self.last_revision = snapshot.revision;

        let controller = self.controller.clone();
        let runtime = self.runtime.clone();
        let selected_run = snapshot.selected_run_id.clone();

        let list = snapshot.runs.iter().take(120).enumerate().fold(
            div().flex().flex_col().gap_1(),
            |list, (index, run)| {
                let run_id = run.run_id.clone();
                let mut button = Button::new(("run", index))
                    .label(format!(
                        "{}  [{}]  {}",
                        run.run_id,
                        format_run_status(&run.status),
                        format_run_time(run)
                    ))
                    .on_click({
                        let controller = controller.clone();
                        let runtime = runtime.clone();
                        move |_, _, _| {
                            let controller = controller.clone();
                            let run_id = run_id.clone();
                            runtime.spawn(async move {
                                let _ = controller
                                    .dispatch_command(DesktopCommand::OpenRun { run_id })
                                    .await;
                            });
                        }
                    });

                if selected_run.as_deref() == Some(run.run_id.as_str()) {
                    button = button.primary();
                }
                list.child(button)
            },
        );

        let route_label = match snapshot.route {
            UiRoute::RunList => "Route: run_list",
            UiRoute::RunDetail => "Route: run_detail",
            UiRoute::WorkflowDetail => "Route: workflow_detail",
        };

        div()
            .id("workdesk-main")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("WorkDesk Studio")
                    .child(route_label),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("refresh")
                            .label("Refresh")
                            .on_click(cx.listener(Self::on_refresh)),
                    )
                    .child(
                        Button::new("cancel-run")
                            .label("Cancel Run")
                            .on_click(cx.listener(Self::on_cancel_run)),
                    )
                    .child(
                        Button::new("retry-run")
                            .label("Retry Run")
                            .on_click(cx.listener(Self::on_retry_run)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .size_full()
                    .gap_2()
                    .child(
                        div()
                            .id("run-list")
                            .flex()
                            .flex_col()
                            .gap_2()
                            .w_1_2()
                            .child("Run List")
                            .child(list),
                    )
                    .child(
                        div()
                            .id("run-detail")
                            .flex()
                            .flex_col()
                            .gap_2()
                            .w_1_2()
                            .child(format!(
                                "Run Detail: {}",
                                snapshot
                                    .selected_run_id
                                    .clone()
                                    .unwrap_or_else(|| "(none)".into())
                            ))
                            .child(render_event_list(&snapshot.run_events))
                            .child(render_skill_list(&snapshot.run_skills)),
                    ),
            )
            .when_some(snapshot.last_error.as_ref(), |div, error| {
                div.child(format!("Error: {error}"))
            })
    }
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
    events.iter().take(100).fold(
        div().flex().flex_col().gap_1().child("Events"),
        |div, event| {
            div.child(format!(
                "#{} {} {}",
                event.seq, event.event_type, event.payload
            ))
        },
    )
}

fn render_skill_list(skills: &[RunSkillSnapshot]) -> impl IntoElement {
    skills.iter().take(100).fold(
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
