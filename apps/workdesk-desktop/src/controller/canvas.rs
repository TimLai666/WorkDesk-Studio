use super::*;
use anyhow::{anyhow, Result};
use workdesk_core::{PatchWorkflowInput, WorkflowNodeInput, WorkflowNodeKind};

impl DesktopAppController {
    pub fn load_canvas_for_selected_workflow(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let workflow_id = snapshot
            .selected_workflow_id
            .ok_or_else(|| anyhow!("no workflow selected"))?;
        let workflow = snapshot
            .workflows
            .iter()
            .find(|workflow| workflow.id == workflow_id)
            .cloned()
            .ok_or_else(|| anyhow!("selected workflow not loaded"))?;
        let (nodes, edges) = canvas_from_workflow(&workflow);
        {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            history.past.clear();
            history.future.clear();
        }
        self.apply(ControllerAction::SetCanvas {
            nodes,
            edges,
            selected: Vec::new(),
        });
        self.apply(ControllerAction::SetCanvasHistoryDepth {
            undo_depth: 0,
            redo_depth: 0,
        });
        Ok(())
    }

    pub fn canvas_add_node(&self, kind: WorkflowNodeKind) {
        self.capture_canvas_for_undo();
        let mut snapshot = self.snapshot();
        let index = snapshot.canvas_nodes.len() as f32;
        let node_id = format!("{:?}_{}", kind, snapshot.canvas_nodes.len() + 1)
            .to_lowercase()
            .replace(' ', "_");
        snapshot.canvas_nodes.push(CanvasNodeState {
            id: node_id.clone(),
            kind,
            x: 80.0 + (index * 140.0),
            y: 120.0 + ((index as i32 % 3) as f32 * 110.0),
        });
        snapshot.selected_canvas_nodes = vec![node_id];
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        });
    }

    pub fn canvas_toggle_selection(&self, node_id: &str, append: bool) {
        let mut snapshot = self.snapshot();
        if !snapshot.canvas_nodes.iter().any(|node| node.id == node_id) {
            return;
        }
        if append {
            if let Some(index) = snapshot
                .selected_canvas_nodes
                .iter()
                .position(|id| id == node_id)
            {
                snapshot.selected_canvas_nodes.remove(index);
            } else {
                snapshot.selected_canvas_nodes.push(node_id.to_string());
            }
        } else {
            snapshot.selected_canvas_nodes = vec![node_id.to_string()];
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        });
    }

    pub fn canvas_select_all(&self) {
        let snapshot = self.snapshot();
        let selected = snapshot
            .canvas_nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected,
        });
    }

    pub fn canvas_clear_selection(&self) {
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: Vec::new(),
        });
    }

    pub fn canvas_connect_selected(&self) {
        let snapshot = self.snapshot();
        if snapshot.selected_canvas_nodes.len() < 2 {
            return;
        }
        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        for pair in updated.selected_canvas_nodes.windows(2) {
            let from = pair[0].clone();
            let to = pair[1].clone();
            let exists = updated
                .canvas_edges
                .iter()
                .any(|edge| edge.from == from && edge.to == to);
            if !exists {
                updated.canvas_edges.push(WorkflowEdge { from, to });
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_distribute_horizontally(&self) {
        let snapshot = self.snapshot();
        if snapshot.selected_canvas_nodes.len() < 3 {
            return;
        }

        let mut selected_nodes = snapshot
            .canvas_nodes
            .iter()
            .filter(|node| {
                snapshot
                    .selected_canvas_nodes
                    .iter()
                    .any(|id| id == &node.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if selected_nodes.len() < 3 {
            return;
        }
        selected_nodes.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let min_x = selected_nodes.first().map(|node| node.x).unwrap_or(0.0);
        let max_x = selected_nodes.last().map(|node| node.x).unwrap_or(0.0);
        if (max_x - min_x).abs() < f32::EPSILON {
            return;
        }
        let step = (max_x - min_x) / (selected_nodes.len() as f32 - 1.0);

        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        for (index, node) in selected_nodes.iter().enumerate() {
            if let Some(target) = updated
                .canvas_nodes
                .iter_mut()
                .find(|item| item.id == node.id)
            {
                target.x = min_x + (step * index as f32);
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_distribute_vertically(&self) {
        let snapshot = self.snapshot();
        if snapshot.selected_canvas_nodes.len() < 3 {
            return;
        }

        let mut selected_nodes = snapshot
            .canvas_nodes
            .iter()
            .filter(|node| {
                snapshot
                    .selected_canvas_nodes
                    .iter()
                    .any(|id| id == &node.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if selected_nodes.len() < 3 {
            return;
        }
        selected_nodes.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
        let min_y = selected_nodes.first().map(|node| node.y).unwrap_or(0.0);
        let max_y = selected_nodes.last().map(|node| node.y).unwrap_or(0.0);
        if (max_y - min_y).abs() < f32::EPSILON {
            return;
        }
        let step = (max_y - min_y) / (selected_nodes.len() as f32 - 1.0);

        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        for (index, node) in selected_nodes.iter().enumerate() {
            if let Some(target) = updated
                .canvas_nodes
                .iter_mut()
                .find(|item| item.id == node.id)
            {
                target.y = min_y + (step * index as f32);
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_move_selected(&self, dx: f32, dy: f32) {
        self.capture_canvas_for_undo();
        let mut snapshot = self.snapshot();
        let selected = snapshot.selected_canvas_nodes.clone();
        for node in &mut snapshot.canvas_nodes {
            if selected.iter().any(|item| item == &node.id) {
                node.x += dx;
                node.y += dy;
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        });
    }

    pub fn canvas_align_left(&self) {
        let snapshot = self.snapshot();
        let selected: Vec<&CanvasNodeState> = snapshot
            .canvas_nodes
            .iter()
            .filter(|node| {
                snapshot
                    .selected_canvas_nodes
                    .iter()
                    .any(|id| id == &node.id)
            })
            .collect();
        if selected.len() < 2 {
            return;
        }
        let target_x = selected
            .iter()
            .map(|node| node.x)
            .fold(f32::INFINITY, f32::min);
        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        let selected = updated.selected_canvas_nodes.clone();
        for node in &mut updated.canvas_nodes {
            if selected.iter().any(|id| id == &node.id) {
                node.x = target_x;
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_align_top(&self) {
        let snapshot = self.snapshot();
        let selected: Vec<&CanvasNodeState> = snapshot
            .canvas_nodes
            .iter()
            .filter(|node| {
                snapshot
                    .selected_canvas_nodes
                    .iter()
                    .any(|id| id == &node.id)
            })
            .collect();
        if selected.len() < 2 {
            return;
        }
        let target_y = selected
            .iter()
            .map(|node| node.y)
            .fold(f32::INFINITY, f32::min);
        self.capture_canvas_for_undo();
        let mut updated = self.snapshot();
        let selected = updated.selected_canvas_nodes.clone();
        for node in &mut updated.canvas_nodes {
            if selected.iter().any(|id| id == &node.id) {
                node.y = target_y;
            }
        }
        self.apply(ControllerAction::SetCanvas {
            nodes: updated.canvas_nodes,
            edges: updated.canvas_edges,
            selected: updated.selected_canvas_nodes,
        });
    }

    pub fn canvas_begin_drag(&self, node_id: &str) {
        if !self
            .snapshot()
            .canvas_nodes
            .iter()
            .any(|node| node.id == node_id)
        {
            return;
        }
        let should_capture = {
            let mut dragging = self
                .canvas_dragging
                .write()
                .expect("canvas dragging write lock");
            if dragging.as_deref() == Some(node_id) {
                false
            } else {
                *dragging = Some(node_id.to_string());
                true
            }
        };
        if should_capture {
            self.capture_canvas_for_undo();
        }

        let mut snapshot = self.snapshot();
        if !snapshot
            .selected_canvas_nodes
            .iter()
            .any(|id| id == node_id)
        {
            snapshot.selected_canvas_nodes = vec![node_id.to_string()];
            self.apply(ControllerAction::SetCanvas {
                nodes: snapshot.canvas_nodes,
                edges: snapshot.canvas_edges,
                selected: snapshot.selected_canvas_nodes,
            });
        }
    }

    pub fn canvas_drag_to(&self, node_id: &str, x: f32, y: f32) {
        let mut snapshot = self.snapshot();
        if let Some(node) = snapshot
            .canvas_nodes
            .iter_mut()
            .find(|node| node.id == node_id)
        {
            node.x = x.max(0.0);
            node.y = y.max(0.0);
            self.apply(ControllerAction::SetCanvas {
                nodes: snapshot.canvas_nodes,
                edges: snapshot.canvas_edges,
                selected: snapshot.selected_canvas_nodes,
            });
        }
    }

    pub fn canvas_end_drag(&self, node_id: &str) {
        let mut dragging = self
            .canvas_dragging
            .write()
            .expect("canvas dragging write lock");
        if dragging.as_deref() == Some(node_id) {
            *dragging = None;
        }
    }

    pub fn canvas_undo(&self) {
        let current = self.current_canvas_snapshot();
        let previous = {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            let Some(previous) = history.past.pop() else {
                return;
            };
            history.future.push(current);
            previous
        };
        self.apply_canvas_snapshot(previous);
        self.refresh_canvas_depth();
    }

    pub fn canvas_redo(&self) {
        let current = self.current_canvas_snapshot();
        let next = {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            let Some(next) = history.future.pop() else {
                return;
            };
            history.past.push(current);
            next
        };
        self.apply_canvas_snapshot(next);
        self.refresh_canvas_depth();
    }

    pub async fn publish_selected_workflow(&self) -> Result<()> {
        self.save_selected_workflow_canvas().await?;
        let workflow_id = self
            .snapshot()
            .selected_workflow_id
            .ok_or_else(|| anyhow!("no workflow selected"))?;
        let _ = self
            .api
            .update_workflow_status(&workflow_id, WorkflowStatus::Active)
            .await?;
        self.refresh_workflows().await?;
        Ok(())
    }

    pub async fn save_selected_workflow_canvas(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let workflow_id = snapshot
            .selected_workflow_id
            .ok_or_else(|| anyhow!("no workflow selected"))?;
        let workflow = snapshot
            .workflows
            .iter()
            .find(|workflow| workflow.id == workflow_id)
            .cloned()
            .ok_or_else(|| anyhow!("selected workflow not loaded"))?;

        let existing_configs = workflow
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.config.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let patch_nodes = snapshot
            .canvas_nodes
            .iter()
            .map(|node| WorkflowNodeInput {
                id: node.id.clone(),
                kind: node.kind.clone(),
                x: Some(node.x),
                y: Some(node.y),
                config: existing_configs.get(&node.id).cloned().flatten(),
            })
            .collect::<Vec<_>>();

        let _ = self
            .api
            .patch_workflow(
                &workflow_id,
                &PatchWorkflowInput {
                    nodes: Some(patch_nodes),
                    edges: Some(snapshot.canvas_edges.clone()),
                    ..PatchWorkflowInput::default()
                },
            )
            .await?;
        self.refresh_workflows().await?;
        self.apply(ControllerAction::SelectWorkflow(Some(workflow_id)));
        self.load_canvas_for_selected_workflow()?;
        Ok(())
    }

    fn capture_canvas_for_undo(&self) {
        let current = self.current_canvas_snapshot();
        {
            let mut history = self
                .canvas_history
                .write()
                .expect("canvas history write lock");
            history.past.push(current);
            if history.past.len() > 100 {
                history.past.remove(0);
            }
            history.future.clear();
        }
        self.refresh_canvas_depth();
    }

    fn current_canvas_snapshot(&self) -> CanvasSnapshot {
        let snapshot = self.snapshot();
        CanvasSnapshot {
            nodes: snapshot.canvas_nodes,
            edges: snapshot.canvas_edges,
            selected: snapshot.selected_canvas_nodes,
        }
    }

    fn apply_canvas_snapshot(&self, snapshot: CanvasSnapshot) {
        self.apply(ControllerAction::SetCanvas {
            nodes: snapshot.nodes,
            edges: snapshot.edges,
            selected: snapshot.selected,
        });
    }

    fn refresh_canvas_depth(&self) {
        let (undo_depth, redo_depth) = {
            let history = self
                .canvas_history
                .read()
                .expect("canvas history read lock");
            (history.past.len(), history.future.len())
        };
        self.apply(ControllerAction::SetCanvasHistoryDepth {
            undo_depth,
            redo_depth,
        });
    }
}

pub(super) fn canvas_from_workflow(
    workflow: &WorkflowDefinition,
) -> (Vec<CanvasNodeState>, Vec<WorkflowEdge>) {
    let nodes = workflow
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| CanvasNodeState {
            id: node.id.clone(),
            kind: node.kind.clone(),
            x: node.x.unwrap_or(80.0 + ((index % 5) as f32 * 180.0)),
            y: node.y.unwrap_or(90.0 + ((index / 5) as f32 * 140.0)),
        })
        .collect();
    (nodes, workflow.edges.clone())
}
