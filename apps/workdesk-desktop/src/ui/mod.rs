
use crate::controller::{DesktopAppController, UiRoute};
use anyhow::Result;
use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, IntoElement, Render, Timer,
    Window, WindowBounds, WindowOptions,
};
use gpui_component::{button::Button, button::ButtonVariants, Root};
use std::time::Duration;
use workdesk_core::{RunStatus, WorkflowRunEvent};

pub fn run_gpui(
    controller: DesktopAppController,
    locale: String,
    runtime: tokio::runtime::Handle,
) -> Result<()> {
    Application::new().run(move |cx: &mut App| {
        gpui_component::set_locale(&locale);
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1320.0), px(860.0)), cx);
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
    last_focus_seq: u64,
}

impl WorkdeskMainView {
    fn new(
        controller: DesktopAppController,
        runtime: tokio::runtime::Handle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |entity, cx| loop {
            Timer::after(Duration::from_millis(500)).await;
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
            last_focus_seq: 0,
        }
    }

    fn on_open_runs(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.navigate(UiRoute::RunList);
    }

    fn on_open_workbench(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.navigate(UiRoute::Workbench);
    }

    fn on_open_canvas(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.navigate(UiRoute::WorkflowDetail);
    }

    fn on_open_files(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.open_file_manager(".").await;
        });
    }

    fn on_open_office(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.navigate(UiRoute::OfficeDesk);
    }

    fn on_refresh(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.refresh_agent_capabilities().await;
            let _ = controller.refresh_agent_sessions().await;
            let _ = controller.refresh_active_agent_workspace().await;
            let _ = controller.refresh_workflows().await;
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
    fn on_canvas_add_schedule(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller
            .canvas_add_node(workdesk_core::WorkflowNodeKind::ScheduleTrigger);
    }

    fn on_canvas_add_agent(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller
            .canvas_add_node(workdesk_core::WorkflowNodeKind::AgentPrompt);
    }

    fn on_canvas_add_code(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller
            .canvas_add_node(workdesk_core::WorkflowNodeKind::CodeExec);
    }

    fn on_canvas_move_right(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_move_selected(24.0, 0.0);
    }

    fn on_canvas_move_down(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_move_selected(0.0, 24.0);
    }

    fn on_canvas_align_left(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_align_left();
    }

    fn on_canvas_undo(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.canvas_undo();
    }

    fn on_canvas_redo(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.canvas_redo();
    }

    fn on_canvas_publish(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.publish_selected_workflow().await;
        });
    }

    fn on_file_open_readme(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.open_file("README.md").await;
        });
    }

    fn on_file_save(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_current_file().await;
        });
    }

    fn on_file_search_todo(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.search_files(".", "TODO").await;
        });
    }

    fn on_file_terminal_dir(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            #[cfg(windows)]
            let command = "dir";
            #[cfg(not(windows))]
            let command = "ls -la";
            let _ = controller.run_terminal(".", command).await;
        });
    }

    fn on_file_diff_self(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.diff_files("README.md", "README.md").await;
        });
    }

    fn on_office_open(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.open_office_document("README.md").await;
        });
    }

    fn on_office_save(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_office_document().await;
        });
    }

    fn on_pdf_preview(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.preview_pdf("README.md").await;
        });
    }

    fn on_pdf_annotate(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.annotate_pdf("annotated in desktop UI").await;
        });
    }

    fn on_pdf_replace(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.replace_pdf_text("TODO", "DONE").await;
        });
    }

    fn on_pdf_save_version(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_pdf_version().await;
        });
    }

    fn on_cycle_model(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.cycle_active_model().await;
        });
    }

    fn on_cycle_reasoning(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.cycle_active_reasoning_effort().await;
        });
    }

    fn on_toggle_speed(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.toggle_active_speed().await;
        });
    }

    fn on_toggle_plan_mode(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.toggle_plan_mode().await;
        });
    }

    fn on_new_file(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.create_new_file_from_workbench().await;
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

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child("WorkDesk Studio")
            .child(format!("route={:?}", snapshot.route));

        let nav = div()
            .flex()
            .gap_2()
            .child(
                Button::new("nav-workbench")
                    .label("Workbench")
                    .on_click(cx.listener(Self::on_open_workbench)),
            )
            .child(Button::new("nav-runs").label("Runs").on_click(cx.listener(Self::on_open_runs)))
            .child(Button::new("nav-canvas").label("Canvas").on_click(cx.listener(Self::on_open_canvas)))
            .child(Button::new("nav-files").label("Files").on_click(cx.listener(Self::on_open_files)))
            .child(Button::new("nav-office").label("Office/PDF").on_click(cx.listener(Self::on_open_office)))
            .child(Button::new("refresh").label("Refresh").on_click(cx.listener(Self::on_refresh)));

        div()
            .id("workdesk-main")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(nav)
            .child(
                match snapshot.route {
                    UiRoute::Workbench => render_workbench(self, cx, &snapshot).into_any_element(),
                    UiRoute::RunList | UiRoute::RunDetail => {
                        render_runs(self, cx, &snapshot).into_any_element()
                    }
                    UiRoute::WorkflowDetail => {
                        render_canvas(self, cx, &snapshot).into_any_element()
                    }
                    UiRoute::FileManager => {
                        render_files(self, cx, &snapshot).into_any_element()
                    }
                    UiRoute::OfficeDesk => {
                        render_office(self, cx, &snapshot).into_any_element()
                    }
                },
            )
            .child(render_diagnostics(&snapshot.diagnostics))
            .when_some(snapshot.last_error.as_ref(), |div, error| {
                div.child(format!("Error: {error}"))
            })
    }
}

fn render_workbench(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let active_session = snapshot.active_agent_session_id.as_ref().and_then(|session_id| {
        snapshot
            .agent_sessions
            .iter()
            .find(|session| session.session_id == *session_id)
    });
    let active_model = active_session
        .and_then(|session| session.config.model.clone())
        .unwrap_or_else(|| "(none)".into());
    let active_reasoning = active_session
        .and_then(|session| session.config.model_reasoning_effort.clone())
        .unwrap_or_else(|| "(none)".into());
    let speed_label = match active_session.and_then(|session| session.config.speed) {
        Some(true) => "Speed: on",
        Some(false) => "Speed: off",
        None => "Speed: unavailable",
    };
    let plan_label = if active_session
        .map(|session| session.config.plan_mode)
        .unwrap_or(false)
    {
        "Plan Mode: on"
    } else {
        "Plan Mode: off"
    };

    let session_list = snapshot.agent_sessions.iter().enumerate().fold(
        div().flex().flex_col().gap_1().child("Sessions"),
        |div, (index, session)| {
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            let session_id = session.session_id.clone();
            let mut button = Button::new(("session", index))
                .label(format!(
                    "{} [{}]",
                    session.title,
                    session.config.model.clone().unwrap_or_else(|| "(model)".into())
                ))
                .on_click(move |_, _, _| {
                    let controller = controller.clone();
                    let session_id = session_id.clone();
                    runtime.spawn(async move {
                        let _ = controller.activate_agent_session(&session_id).await;
                    });
                });
            if snapshot.active_agent_session_id.as_deref() == Some(session.session_id.as_str()) {
                button = button.primary();
            }
            div.child(button)
        },
    );

    let messages = snapshot.agent_messages.iter().fold(
        div().flex().flex_col().gap_1().child("Messages"),
        |div, message| div.child(format!("{:?}: {}", message.role, message.content)),
    );

    let capabilities = snapshot.model_capabilities.iter().take(8).fold(
        div().flex().flex_col().gap_1().child("Capabilities"),
        |div, capability| {
            div.child(format!(
                "{} [{}]",
                capability.display_name,
                capability
                    .reasoning_values
                    .iter()
                    .map(|value| value.reasoning_effort.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        },
    );

    let choice_prompt = match &snapshot.pending_choice_prompt {
        Some(prompt) => {
            let prompt_view = prompt.options.iter().enumerate().fold(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("Choice Prompt: {}", prompt.question)),
                |div, (index, option)| {
                    let controller = view.controller.clone();
                    let runtime = view.runtime.clone();
                    let session_id = prompt.session_id.clone();
                    let prompt_id = prompt.prompt_id.clone();
                    let option_id = option.option_id.clone();
                    let mut button = Button::new(("prompt-option", index))
                        .label(format!("{} - {}", option.label, option.description))
                        .on_click(move |_, _, _| {
                            let controller = controller.clone();
                            let session_id = session_id.clone();
                            let prompt_id = prompt_id.clone();
                            let option_id = option_id.clone();
                            runtime.spawn(async move {
                                let _ = controller
                                    .answer_choice_prompt_option(&session_id, &prompt_id, &option_id)
                                    .await;
                            });
                        });
                    if prompt.recommended_option_id.as_deref() == Some(option.option_id.as_str()) {
                        button = button.primary();
                    }
                    div.child(button)
                },
            );
            prompt_view.into_any_element()
        }
        None => div().child("Choice Prompt: (none)").into_any_element(),
    };

    div()
        .flex()
        .size_full()
        .gap_3()
        .child(div().flex().flex_col().gap_2().w_1_5().child(session_list).child(capabilities))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_2_5()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(Button::new("model-cycle").label(format!("Model: {active_model}")).on_click(cx.listener(WorkdeskMainView::on_cycle_model)))
                        .child(Button::new("reasoning-cycle").label(format!("Reasoning: {active_reasoning}")).on_click(cx.listener(WorkdeskMainView::on_cycle_reasoning)))
                        .child(Button::new("speed-toggle").label(speed_label).on_click(cx.listener(WorkdeskMainView::on_toggle_speed)))
                        .child(Button::new("plan-toggle").label(plan_label).on_click(cx.listener(WorkdeskMainView::on_toggle_plan_mode)))
                        .child(Button::new("new-file").label("New File").on_click(cx.listener(WorkdeskMainView::on_new_file))),
                )
                .child(messages)
                .child(choice_prompt),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_2_5()
                .child("Context Panels")
                .child(render_runs(view, cx, snapshot))
                .child(render_files(view, cx, snapshot)),
        )
}

fn render_runs(
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
                        .child(Button::new("cancel-run").label("Cancel").on_click(cx.listener(WorkdeskMainView::on_cancel_run)))
                        .child(Button::new("retry-run").label("Retry").on_click(cx.listener(WorkdeskMainView::on_retry_run))),
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

fn render_canvas(
    _view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let workflow_label = snapshot
        .selected_workflow_id
        .clone()
        .unwrap_or_else(|| "(select workflow via CLI open-workflow)".into());
    let nodes = snapshot.canvas_nodes.iter().fold(
        div().flex().flex_col().gap_1(),
        |div, node| {
            let selected = snapshot
                .selected_canvas_nodes
                .iter()
                .any(|item| item == &node.id);
            let label = format!(
                "{} {:?} ({:.0}, {:.0}){}",
                node.id,
                node.kind,
                node.x,
                node.y,
                if selected { " [selected]" } else { "" }
            );
            div.child(label)
        },
    );

    let edges = snapshot.canvas_edges.iter().fold(
        div().flex().flex_col().gap_1().child("Edges"),
        |div, edge| div.child(format!("{} -> {}", edge.from, edge.to)),
    );

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(format!("Workflow Canvas: {workflow_label}"))
        .child(
            div()
                .flex()
                .gap_2()
                .child(Button::new("canvas-schedule").label("Add Schedule").on_click(cx.listener(WorkdeskMainView::on_canvas_add_schedule)))
                .child(Button::new("canvas-agent").label("Add Agent").on_click(cx.listener(WorkdeskMainView::on_canvas_add_agent)))
                .child(Button::new("canvas-code").label("Add Code").on_click(cx.listener(WorkdeskMainView::on_canvas_add_code)))
                .child(Button::new("canvas-right").label("Move Right").on_click(cx.listener(WorkdeskMainView::on_canvas_move_right)))
                .child(Button::new("canvas-down").label("Move Down").on_click(cx.listener(WorkdeskMainView::on_canvas_move_down)))
                .child(Button::new("canvas-align").label("Align Left").on_click(cx.listener(WorkdeskMainView::on_canvas_align_left)))
                .child(Button::new("canvas-undo").label("Undo").on_click(cx.listener(WorkdeskMainView::on_canvas_undo)))
                .child(Button::new("canvas-redo").label("Redo").on_click(cx.listener(WorkdeskMainView::on_canvas_redo)))
                .child(Button::new("canvas-publish").label("Publish").on_click(cx.listener(WorkdeskMainView::on_canvas_publish))),
        )
        .child(format!(
            "Mini-map: nodes={} edges={} undo={} redo={}",
            snapshot.canvas_nodes.len(),
            snapshot.canvas_edges.len(),
            snapshot.canvas_undo_depth,
            snapshot.canvas_redo_depth
        ))
        .child(div().flex().gap_3().child(nodes).child(edges))
        .child(format!(
            "Selected: {}",
            if snapshot.selected_canvas_nodes.is_empty() {
                "(none)".into()
            } else {
                snapshot.selected_canvas_nodes.join(", ")
            }
        ))
        .child(format!(
            "Loaded workflows: {}",
            snapshot
                .workflows
                .iter()
                .map(|workflow| format!("{} v{} {:?}", workflow.id, workflow.version, workflow.status))
                .collect::<Vec<_>>()
                .join(" | ")
        ))
}

fn render_files(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let entries = snapshot.workspace_entries.iter().fold(
        div().flex().flex_col().gap_1(),
        |div, entry| {
            let prefix = if entry.is_dir { "[dir]" } else { "[file]" };
            div.child(format!("{prefix} {}", entry.path))
        },
    );
    let search = snapshot.file_search_results.iter().fold(
        div().flex().flex_col().gap_1().child("Search"),
        |div, item| div.child(format!("{}:{} {}", item.path, item.line, item.preview)),
    );
    let diff = match &snapshot.diff_result {
        Some(diff) => diff.hunks.iter().take(20).fold(
            div().flex().flex_col().gap_1().child("Diff"),
            |div, line| div.child(format!("{} {:?}/{:?} {}", line.kind, line.left_line, line.right_line, line.text)),
        ),
        None => div().child("Diff: (none)"),
    };
    let terminal = snapshot
        .terminal_session
        .as_ref()
        .map(|session| {
            format!(
                "Terminal {} [{}] exit={:?}\nstdout:\n{}\nstderr:\n{}",
                session.session_id, session.status, session.exit_code, session.stdout, session.stderr
            )
        })
        .unwrap_or_else(|| "Terminal: (none)".into());

    let _ = view;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .child(Button::new("file-open-readme").label("Open README").on_click(cx.listener(WorkdeskMainView::on_file_open_readme)))
                .child(Button::new("file-save").label("Save").on_click(cx.listener(WorkdeskMainView::on_file_save)))
                .child(Button::new("file-search").label("Search TODO").on_click(cx.listener(WorkdeskMainView::on_file_search_todo)))
                .child(Button::new("file-diff").label("Self Diff").on_click(cx.listener(WorkdeskMainView::on_file_diff_self)))
                .child(Button::new("file-terminal").label("Terminal").on_click(cx.listener(WorkdeskMainView::on_file_terminal_dir))),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .w_1_2()
                        .child("Workspace")
                        .child(entries),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .w_1_2()
                        .child(format!(
                            "Editor: {}",
                            snapshot
                                .current_file_path
                                .clone()
                                .unwrap_or_else(|| "(none)".into())
                        ))
                        .child(snapshot.current_file_content.clone()),
                ),
        )
        .child(search)
        .child(diff)
        .child(terminal)
}

fn render_office(
    _view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let versions = snapshot.office_versions.iter().fold(
        div().flex().flex_col().gap_1().child("Versions"),
        |div, version| div.child(version.clone()),
    );
    let pdf_op = snapshot
        .pdf_last_operation
        .as_ref()
        .map(|operation| {
            format!(
                "PDF op: path={} replaced={} version={}",
                operation.path, operation.replaced_count, operation.version_name
            )
        })
        .unwrap_or_else(|| "PDF op: (none)".into());

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .child(Button::new("office-open").label("Open README").on_click(cx.listener(WorkdeskMainView::on_office_open)))
                .child(Button::new("office-save").label("Save Office").on_click(cx.listener(WorkdeskMainView::on_office_save)))
                .child(Button::new("pdf-preview").label("Preview PDF").on_click(cx.listener(WorkdeskMainView::on_pdf_preview)))
                .child(Button::new("pdf-annotate").label("Annotate").on_click(cx.listener(WorkdeskMainView::on_pdf_annotate)))
                .child(Button::new("pdf-replace").label("Replace").on_click(cx.listener(WorkdeskMainView::on_pdf_replace)))
                .child(Button::new("pdf-version").label("Save Version").on_click(cx.listener(WorkdeskMainView::on_pdf_save_version))),
        )
        .child(format!(
            "Document: {}",
            snapshot.office_path.clone().unwrap_or_else(|| "(none)".into())
        ))
        .child(snapshot.office_editor_text.clone())
        .child(versions)
        .child(pdf_op)
}

fn render_diagnostics(diagnostics: &[crate::controller::UiDiagnostic]) -> impl IntoElement {
    diagnostics.iter().fold(
        div().flex().flex_col().gap_1().child("Diagnostics"),
        |div, item| {
            let target = item
                .run_id
                .as_ref()
                .map(|id| format!(" run={id}"))
                .unwrap_or_default();
            div.child(format!("{}:{}{}", item.code, item.message, target))
        },
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
        |div, event| div.child(format!("#{} {} {}", event.seq, event.event_type, event.payload)),
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
