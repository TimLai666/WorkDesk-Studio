use super::*;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

impl DesktopAppController {
    pub async fn open_file_manager(&self, root: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::FileManager));
        let entries = self.api.fs_tree(root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn open_file(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::FileManager));
        let file = self.api.fs_read(path).await?;
        let raw = STANDARD.decode(file.content_base64.as_bytes())?;
        let text = String::from_utf8_lossy(&raw).to_string();
        self.apply(ControllerAction::SetCurrentFile {
            path: Some(path.to_string()),
            content: text,
        });
        Ok(())
    }

    pub async fn save_current_file(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let path = snapshot
            .current_file_path
            .ok_or_else(|| anyhow!("no file selected"))?;
        let content_base64 = STANDARD.encode(snapshot.current_file_content.as_bytes());
        self.api.fs_write(&path, content_base64).await?;
        Ok(())
    }

    pub fn set_current_file_content(&self, content: String) {
        let path = self.snapshot().current_file_path;
        self.apply(ControllerAction::SetCurrentFile { path, content });
    }

    pub async fn create_file(&self, path: &str, content: &str) -> Result<()> {
        self.api
            .fs_write(path, STANDARD.encode(content.as_bytes()))
            .await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        self.api.fs_move(from, to).await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn delete_path(&self, path: &str) -> Result<()> {
        self.api.fs_delete(path).await?;
        let root = self.workspace_root_from_entries();
        let entries = self.api.fs_tree(&root).await?;
        self.apply(ControllerAction::SetWorkspaceEntries(entries));
        Ok(())
    }

    pub async fn search_files(&self, root: &str, query: &str) -> Result<()> {
        let results = self.api.fs_search(root, query, 500).await?;
        self.apply(ControllerAction::SetFileSearchResults(results));
        Ok(())
    }

    pub async fn diff_files(&self, left_path: &str, right_path: &str) -> Result<()> {
        let diff = self.api.fs_diff(left_path, right_path).await?;
        self.apply(ControllerAction::SetDiffResult(Some(diff)));
        Ok(())
    }

    pub async fn run_terminal(&self, path: &str, command: &str) -> Result<()> {
        let session = self
            .api
            .terminal_start(&TerminalStartInput {
                path: path.to_string(),
                command: command.to_string(),
            })
            .await?;
        let session = self.api.terminal_session(&session.session_id).await?;
        self.apply(ControllerAction::SetTerminalSession(Some(session)));
        Ok(())
    }

    pub(super) fn workspace_root_from_entries(&self) -> String {
        let snapshot = self.snapshot();
        if snapshot.workspace_entries.is_empty() {
            ".".to_string()
        } else {
            snapshot
                .workspace_entries
                .iter()
                .map(|entry| entry.path.as_str())
                .min()
                .unwrap_or(".")
                .to_string()
        }
    }
}
