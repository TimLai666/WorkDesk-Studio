use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopCommand {
    Open,
    OpenRun { run_id: String },
    OpenWorkflow { workflow_id: String },
    RunWorkflow { workflow_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCli {
    pub command: DesktopCommand,
    pub remote_mode: bool,
    pub automation_mode: bool,
}

impl DesktopCli {
    pub fn parse_from<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut remote_mode = false;
        let mut automation_mode = false;
        let mut positional = Vec::new();
        let mut iter = args.into_iter().map(Into::into);
        let _binary = iter.next();

        for arg in iter {
            match arg.as_str() {
                "--remote" => remote_mode = true,
                "--automation" => automation_mode = true,
                _ if arg.starts_with("--") => {
                    positional.push(arg);
                }
                _ => positional.push(arg),
            }
        }

        let command = parse_desktop_command(&positional)?;
        Ok(Self {
            command,
            remote_mode,
            automation_mode,
        })
    }
}

fn parse_desktop_command(args: &[String]) -> Result<DesktopCommand> {
    if args.is_empty() {
        return Ok(DesktopCommand::Open);
    }

    let command = args[0].as_str();
    let tail = &args[1..];
    match command {
        "open" => {
            ensure_no_extra_options("open", tail)?;
            Ok(DesktopCommand::Open)
        }
        "open-run" => Ok(DesktopCommand::OpenRun {
            run_id: require_option_value("--run-id", tail)?,
        }),
        "open-workflow" => Ok(DesktopCommand::OpenWorkflow {
            workflow_id: require_option_value("--workflow-id", tail)?,
        }),
        "run-workflow" => Ok(DesktopCommand::RunWorkflow {
            workflow_id: require_option_value("--workflow-id", tail)?,
        }),
        other => Err(anyhow!(
            "unknown command `{other}`; expected one of: open, open-run, open-workflow, run-workflow"
        )),
    }
}

fn ensure_no_extra_options(command: &str, args: &[String]) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "command `{command}` does not accept extra arguments: {}",
            args.join(" ")
        ))
    }
}

fn require_option_value(name: &str, args: &[String]) -> Result<String> {
    let mut index = 0usize;
    while index < args.len() {
        if args[index] == name {
            if index + 1 >= args.len() {
                return Err(anyhow!("missing value for option `{name}`"));
            }
            let value = args[index + 1].clone();
            if value.starts_with("--") {
                return Err(anyhow!("option `{name}` requires a value, found `{value}`"));
            }
            return Ok(value);
        }
        index += 1;
    }
    Err(anyhow!("missing required option `{name}`"))
}

#[cfg(test)]
mod tests {
    use super::{DesktopCli, DesktopCommand};

    #[test]
    fn parse_defaults_to_open() {
        let cli = DesktopCli::parse_from(["workdesk-desktop"]).expect("parse");
        assert_eq!(cli.command, DesktopCommand::Open);
        assert!(!cli.remote_mode);
        assert!(!cli.automation_mode);
    }

    #[test]
    fn parse_open_run() {
        let cli = DesktopCli::parse_from([
            "workdesk-desktop",
            "open-run",
            "--run-id",
            "run-123",
            "--automation",
        ])
        .expect("parse");
        assert_eq!(
            cli.command,
            DesktopCommand::OpenRun {
                run_id: "run-123".into()
            }
        );
        assert!(cli.automation_mode);
    }

    #[test]
    fn parse_run_workflow() {
        let cli = DesktopCli::parse_from([
            "workdesk-desktop",
            "--remote",
            "run-workflow",
            "--workflow-id",
            "wf-001",
        ])
        .expect("parse");
        assert_eq!(
            cli.command,
            DesktopCommand::RunWorkflow {
                workflow_id: "wf-001".into()
            }
        );
        assert!(cli.remote_mode);
    }
}
