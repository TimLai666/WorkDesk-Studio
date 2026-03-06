use super::*;
use anyhow::{anyhow, Result};

impl DesktopAppController {
    pub async fn refresh_runs(&self) -> Result<()> {
        let runs = self.api.list_runs(200).await?;
        self.apply(ControllerAction::SetRuns(runs));
        self.sync_diagnostics();
        Ok(())
    }

    pub async fn refresh_workflows(&self) -> Result<()> {
        let workflows = self.api.list_workflows().await?;
        self.apply(ControllerAction::SetWorkflows(workflows));
        Ok(())
    }

    pub async fn refresh_selected_run_detail(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        self.refresh_run_detail(&run_id).await
    }

    pub async fn cancel_selected_run(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        self.api.cancel_run(&run_id, Some("desktop-ui")).await?;
        self.refresh_runs().await?;
        self.refresh_run_detail(&run_id).await?;
        Ok(())
    }

    pub async fn retry_selected_run(&self) -> Result<()> {
        let run_id = self
            .snapshot()
            .selected_run_id
            .ok_or_else(|| anyhow!("no run selected"))?;
        let retry = self.api.retry_run(&run_id, Some("desktop-ui")).await?;
        self.apply(ControllerAction::SelectRun(Some(retry.run_id.clone())));
        self.apply(ControllerAction::SetRoute(UiRoute::RunDetail));
        self.refresh_runs().await?;
        self.refresh_run_detail(&retry.run_id).await?;
        Ok(())
    }

    pub(super) async fn refresh_run_detail(&self, run_id: &str) -> Result<()> {
        let events = self.api.list_run_events(run_id, 0, 2000).await?;
        let nodes = self.api.list_run_nodes(run_id).await?;
        let skills = self.api.list_run_skills(run_id).await?;
        self.apply(ControllerAction::SetRunDetails {
            events,
            nodes,
            skills,
        });
        Ok(())
    }
}
