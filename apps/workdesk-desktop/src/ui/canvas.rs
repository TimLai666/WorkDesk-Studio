use super::*;
use gpui::{DragMoveEvent, Render};

#[derive(Debug, Clone)]
pub(crate) struct CanvasNodeDrag {
    pub node_id: String,
}

#[derive(Debug, Clone)]
struct CanvasDragPreview {
    label: String,
}

impl Render for CanvasDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(gpui::hsla(0.58, 0.55, 0.45, 0.95))
            .text_color(gpui::hsla(0.0, 0.0, 1.0, 1.0))
            .child(format!("Dragging {}", self.label))
    }
}

pub(super) fn render_canvas(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let workflow_label = snapshot
        .selected_workflow_id
        .clone()
        .unwrap_or_else(|| "(select workflow via CLI open-workflow)".into());
    let edges = snapshot.canvas_edges.iter().fold(
        div().flex().flex_col().gap_1().child("Edges"),
        |div, edge| div.child(format!("{} -> {}", edge.from, edge.to)),
    );

    let canvas_surface = snapshot.canvas_nodes.iter().enumerate().fold(
        div()
            .id("canvas-surface")
            .relative()
            .w_full()
            .h(px(520.0))
            .rounded_md()
            .bg(gpui::hsla(0.58, 0.22, 0.09, 1.0))
            .on_drag_move(cx.listener(WorkdeskMainView::on_canvas_drag_move))
            .on_drop(cx.listener(WorkdeskMainView::on_canvas_drag_drop))
            .on_key_down(cx.listener(WorkdeskMainView::on_canvas_key_down)),
        |surface, (index, node)| {
            let selected = snapshot
                .selected_canvas_nodes
                .iter()
                .any(|item| item == &node.id);
            let label = format!(
                "{} {:?}\n({:.0}, {:.0})",
                node.id, node.kind, node.x, node.y
            );
            let controller = view.controller.clone();
            let node_id = node.id.clone();
            let node_drag = CanvasNodeDrag {
                node_id: node.id.clone(),
            };

            let mut button =
                Button::new(("canvas-node", index))
                    .label(label)
                    .on_click(move |_, _, _| {
                        controller.canvas_toggle_selection(&node_id, true);
                    });
            if selected {
                button = button.primary();
            }

            surface.child(
                div()
                    .id(("canvas-node-absolute", index))
                    .absolute()
                    .left(px(node.x))
                    .top(px(node.y))
                    .w(px(180.0))
                    .h(px(56.0))
                    .on_drag(node_drag, |drag, _, _, cx| {
                        cx.new(|_| CanvasDragPreview {
                            label: drag.node_id.clone(),
                        })
                    })
                    .child(button),
            )
        },
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
                .child(
                    Button::new("canvas-schedule")
                        .label("Add Schedule")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_add_schedule)),
                )
                .child(
                    Button::new("canvas-agent")
                        .label("Add Agent")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_add_agent)),
                )
                .child(
                    Button::new("canvas-code")
                        .label("Add Code")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_add_code)),
                )
                .child(
                    Button::new("canvas-right")
                        .label("Move Right")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_move_right)),
                )
                .child(
                    Button::new("canvas-down")
                        .label("Move Down")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_move_down)),
                )
                .child(
                    Button::new("canvas-align")
                        .label("Align Left")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_align_left)),
                )
                .child(
                    Button::new("canvas-align-top")
                        .label("Align Top")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_align_top)),
                )
                .child(
                    Button::new("canvas-distribute-h")
                        .label("Distribute H")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_distribute_horizontal)),
                )
                .child(
                    Button::new("canvas-distribute-v")
                        .label("Distribute V")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_distribute_vertical)),
                )
                .child(
                    Button::new("canvas-connect")
                        .label("Connect Selected")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_connect_selected)),
                )
                .child(
                    Button::new("canvas-select-all")
                        .label("Select All")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_select_all)),
                )
                .child(
                    Button::new("canvas-clear")
                        .label("Clear Selection")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_clear_selection)),
                )
                .child(
                    Button::new("canvas-undo")
                        .label("Undo")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_undo)),
                )
                .child(
                    Button::new("canvas-redo")
                        .label("Redo")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_redo)),
                )
                .child(
                    Button::new("canvas-save")
                        .label("Save")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_save)),
                )
                .child(
                    Button::new("canvas-publish")
                        .label("Publish")
                        .on_click(cx.listener(WorkdeskMainView::on_canvas_publish)),
                ),
        )
        .child(format!(
            "Mini-map: nodes={} edges={} undo={} redo={} | shortcuts: Ctrl+Z / Ctrl+Shift+Z / arrows",
            snapshot.canvas_nodes.len(),
            snapshot.canvas_edges.len(),
            snapshot.canvas_undo_depth,
            snapshot.canvas_redo_depth
        ))
        .child(canvas_surface)
        .child(edges)
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
                .map(|workflow| format!(
                    "{} v{} {:?}",
                    workflow.id, workflow.version, workflow.status
                ))
                .collect::<Vec<_>>()
                .join(" | ")
        ))
}

pub(crate) fn drag_target_position(event: &DragMoveEvent<CanvasNodeDrag>) -> (f32, f32) {
    let x: f32 = (event.event.position.x - event.bounds.left()).into();
    let y: f32 = (event.event.position.y - event.bounds.top()).into();
    let x = x - 90.0;
    let y = y - 24.0;
    (x.max(0.0), y.max(0.0))
}
