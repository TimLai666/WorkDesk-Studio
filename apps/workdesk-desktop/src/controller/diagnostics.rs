use super::*;
use chrono::Utc;

impl DesktopAppController {
    pub fn set_runtime_diagnostic(&self, source: &str, diagnostic: Option<UiDiagnostic>) {
        {
            let mut runtime = self
                .runtime_diagnostics
                .write()
                .expect("runtime diagnostics write lock");
            if let Some(item) = diagnostic {
                runtime.insert(source.to_string(), item);
            } else {
                runtime.remove(source);
            }
        }
        self.sync_diagnostics();
    }

    pub(super) fn sync_diagnostics(&self) {
        let runs = self.snapshot().runs;
        let mut diagnostics = Self::derive_run_diagnostics(&runs);
        diagnostics.extend(
            self.runtime_diagnostics
                .read()
                .expect("runtime diagnostics read lock")
                .values()
                .cloned(),
        );
        diagnostics.sort_by(|a, b| a.code.cmp(&b.code).then(a.message.cmp(&b.message)));
        self.apply(ControllerAction::SetDiagnostics(diagnostics));
    }

    fn derive_run_diagnostics(runs: &[WorkflowRun]) -> Vec<UiDiagnostic> {
        let now = Utc::now();
        runs.iter()
            .filter_map(|run| {
                let queued_too_long = matches!(run.status, workdesk_core::RunStatus::Queued)
                    && (now - run.created_at).num_seconds() >= 90;
                queued_too_long.then(|| UiDiagnostic {
                    code: "RUNNER_UNAVAILABLE".to_string(),
                    message: format!(
                        "Run {} has been queued for over 90 seconds. Check runner process status.",
                        run.run_id
                    ),
                    run_id: Some(run.run_id.clone()),
                })
            })
            .collect()
    }
}
