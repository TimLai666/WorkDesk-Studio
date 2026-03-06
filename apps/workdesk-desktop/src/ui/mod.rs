mod canvas;
mod diagnostics;
mod files;
mod office;
mod runs;
pub(crate) mod widgets;
mod workbench;

use crate::controller::{DesktopAppController, UiRoute};
use anyhow::Result;
use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Context, DragMoveEvent, Entity,
    IntoElement, KeyDownEvent, Render, Timer, Window, WindowBounds, WindowOptions,
};
use gpui_component::input::InputState;
use gpui_component::{button::Button, button::ButtonVariants, Root};
#[cfg(feature = "webview")]
use gpui_component::{webview::WebView, wry};
use std::time::Duration;

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
                        window,
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

pub(crate) struct WorkdeskMainView {
    pub(crate) controller: DesktopAppController,
    pub(crate) runtime: tokio::runtime::Handle,
    pub(crate) prompt_input: Entity<InputState>,
    #[cfg(feature = "webview")]
    pub(crate) office_webview: Option<Entity<WebView>>,
    #[cfg(feature = "webview")]
    office_webview_url: Option<String>,
    last_focus_seq: u64,
}

impl WorkdeskMainView {
    fn new(
        controller: DesktopAppController,
        runtime: tokio::runtime::Handle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let prompt_input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask WorkDesk..."));
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
            prompt_input,
            #[cfg(feature = "webview")]
            office_webview: None,
            #[cfg(feature = "webview")]
            office_webview_url: None,
            last_focus_seq: 0,
        }
    }

    #[cfg(feature = "webview")]
    pub(crate) fn ensure_office_webview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        url: &str,
    ) -> Option<Entity<WebView>> {
        if let Some(existing) = self.office_webview.clone() {
            if self.office_webview_url.as_deref() != Some(url) {
                let load_url = url.to_string();
                let _ = existing.update(cx, |webview, _| {
                    webview.load_url(&load_url);
                    webview.show();
                });
                self.office_webview_url = Some(url.to_string());
            } else {
                let _ = existing.update(cx, |webview, _| {
                    webview.show();
                });
            }
            return Some(existing);
        }

        let builder = wry::WebViewBuilder::new().with_url(url);
        let webview = builder.build_as_child(window).ok()?;
        let entity = cx.new(|cx| WebView::new(webview, window, cx));
        self.office_webview = Some(entity.clone());
        self.office_webview_url = Some(url.to_string());
        Some(entity)
    }

    #[cfg(feature = "webview")]
    pub(crate) fn hide_office_webview(&mut self, cx: &mut Context<Self>) {
        if let Some(webview) = self.office_webview.clone() {
            let _ = webview.update(cx, |webview, _| {
                webview.hide();
            });
        }
    }

    #[cfg(not(feature = "webview"))]
    pub(crate) fn ensure_office_webview(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
        _url: &str,
    ) -> Option<Entity<gpui::Empty>> {
        None
    }

    #[cfg(not(feature = "webview"))]
    pub(crate) fn hide_office_webview(&mut self, _cx: &mut Context<Self>) {}

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
    fn on_canvas_add_schedule(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
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

    fn on_canvas_move_down(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
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

    fn on_canvas_distribute_horizontal(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_distribute_horizontally();
    }

    fn on_canvas_align_top(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.canvas_align_top();
    }

    fn on_canvas_distribute_vertical(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_distribute_vertically();
    }

    fn on_canvas_connect_selected(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_connect_selected();
    }

    fn on_canvas_select_all(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_select_all();
    }

    fn on_canvas_clear_selection(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_clear_selection();
    }

    fn on_canvas_undo(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.canvas_undo();
    }

    fn on_canvas_redo(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        self.controller.canvas_redo();
    }

    fn on_canvas_publish(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.publish_selected_workflow().await;
        });
    }

    fn on_canvas_save(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_selected_workflow_canvas().await;
        });
    }

    fn on_canvas_drag_move(
        &mut self,
        event: &DragMoveEvent<canvas::CanvasNodeDrag>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        self.controller.canvas_begin_drag(&drag.node_id);
        let (x, y) = canvas::drag_target_position(event);
        self.controller.canvas_drag_to(&drag.node_id, x, y);
    }

    fn on_canvas_drag_drop(
        &mut self,
        drag: &canvas::CanvasNodeDrag,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.controller.canvas_end_drag(&drag.node_id);
    }

    fn on_canvas_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, _: &mut Context<Self>) {
        let key = event.keystroke.key.to_ascii_lowercase();
        let modifiers = event.keystroke.modifiers;
        if modifiers.secondary() && key == "z" && modifiers.shift {
            self.controller.canvas_redo();
            return;
        }
        if modifiers.secondary() && key == "z" {
            self.controller.canvas_undo();
            return;
        }
        match key.as_str() {
            "arrowleft" => self.controller.canvas_move_selected(-18.0, 0.0),
            "arrowright" => self.controller.canvas_move_selected(18.0, 0.0),
            "arrowup" => self.controller.canvas_move_selected(0.0, -18.0),
            "arrowdown" => self.controller.canvas_move_selected(0.0, 18.0),
            _ => {}
        }
    }

    fn on_file_open_readme(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
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

    fn on_file_search_todo(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
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

    fn on_office_open(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        let path = {
            let value = self.prompt_input.read(cx).value().to_string();
            let value = value.trim().to_string();
            if value.is_empty() {
                "README.md".to_string()
            } else {
                self.prompt_input.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                });
                value
            }
        };
        self.runtime.spawn(async move {
            let _ = controller.open_office_document(&path).await;
        });
    }

    fn on_office_save(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_office_document().await;
        });
    }

    fn on_pdf_preview(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let controller = self.controller.clone();
        let path = {
            let value = self.prompt_input.read(cx).value().to_string();
            let value = value.trim().to_string();
            if value.is_empty() {
                "README.md".to_string()
            } else {
                self.prompt_input.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                });
                value
            }
        };
        self.runtime.spawn(async move {
            let _ = controller.preview_pdf(&path).await;
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

    fn on_pdf_save_version(&mut self, _: &gpui::ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        let controller = self.controller.clone();
        self.runtime.spawn(async move {
            let _ = controller.save_pdf_version().await;
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
        if !matches!(snapshot.route, UiRoute::OfficeDesk) {
            self.hide_office_webview(cx);
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
            .child(
                Button::new("nav-runs")
                    .label("Runs")
                    .on_click(cx.listener(Self::on_open_runs)),
            )
            .child(
                Button::new("nav-canvas")
                    .label("Canvas")
                    .on_click(cx.listener(Self::on_open_canvas)),
            )
            .child(
                Button::new("nav-files")
                    .label("Files")
                    .on_click(cx.listener(Self::on_open_files)),
            )
            .child(
                Button::new("nav-office")
                    .label("Office/PDF")
                    .on_click(cx.listener(Self::on_open_office)),
            )
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .on_click(cx.listener(Self::on_refresh)),
            );

        div()
            .id("workdesk-main")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(nav)
            .child(match snapshot.route {
                UiRoute::Workbench => {
                    workbench::render_workbench(self, cx, &snapshot).into_any_element()
                }
                UiRoute::RunList | UiRoute::RunDetail => {
                    runs::render_runs(self, cx, &snapshot).into_any_element()
                }
                UiRoute::WorkflowDetail => {
                    canvas::render_canvas(self, cx, &snapshot).into_any_element()
                }
                UiRoute::FileManager => files::render_files(self, cx, &snapshot).into_any_element(),
                UiRoute::OfficeDesk => {
                    office::render_office(self, window, cx, &snapshot).into_any_element()
                }
            })
            .child(diagnostics::render_diagnostics(&snapshot.diagnostics))
            .when_some(snapshot.last_error.as_ref(), |div, error| {
                div.child(format!("Error: {error}"))
            })
    }
}
