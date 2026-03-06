use super::*;
use gpui_component::input::Input;

pub(super) fn render_workbench(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let active_session = snapshot
        .active_agent_session_id
        .as_ref()
        .and_then(|session_id| {
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
    let active_capability = active_session.and_then(|session| {
        snapshot
            .model_capabilities
            .iter()
            .find(|capability| session.config.model.as_deref() == Some(capability.model.as_str()))
    });
    let speed_enabled = active_session
        .and_then(|session| session.config.speed)
        .unwrap_or(false);
    let plan_enabled = active_session
        .map(|session| session.config.plan_mode)
        .unwrap_or(false);
    let auth_label = snapshot
        .auth_account_id
        .clone()
        .map(|account| format!("Auth: {account}"))
        .unwrap_or_else(|| "Auth: (none)".into());

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
                    session
                        .config
                        .model
                        .clone()
                        .unwrap_or_else(|| "(model)".into())
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
    let session_list = session_list.child({
        let controller = view.controller.clone();
        let runtime = view.runtime.clone();
        Button::new("session-create")
            .label("Create Session")
            .on_click(move |_, _, _| {
                let controller = controller.clone();
                runtime.spawn(async move {
                    let _ = controller.create_agent_session("New Session").await;
                });
            })
    });
    let session_list = session_list
        .child(auth_label)
        .child({
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            let prompt_input = view.prompt_input.clone();
            Button::new("auth-login")
                .label("Login")
                .on_click(move |_, window, cx| {
                    let raw = prompt_input.read(cx).value().to_string();
                    let raw = raw.trim().to_string();
                    if raw.is_empty() {
                        return;
                    }
                    let (account, password) = raw
                        .split_once(':')
                        .map(|(account, password)| {
                            (account.trim().to_string(), password.trim().to_string())
                        })
                        .unwrap_or_else(|| ("local".to_string(), raw));
                    prompt_input.update(cx, |input, cx| {
                        input.set_value("", window, cx);
                    });
                    let controller = controller.clone();
                    runtime.spawn(async move {
                        let _ = controller.login_local_account(&account, &password).await;
                    });
                })
        })
        .child({
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            Button::new("auth-logout")
                .label("Logout")
                .on_click(move |_, _, _| {
                    let controller = controller.clone();
                    runtime.spawn(async move {
                        let _ = controller.logout_active_account().await;
                    });
                })
        })
        .child({
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            let prompt_input = view.prompt_input.clone();
            let current_account = snapshot.auth_account_id.clone();
            Button::new("auth-switch")
                .label("Switch")
                .on_click(move |_, window, cx| {
                    let raw = prompt_input.read(cx).value().to_string();
                    let raw = raw.trim().to_string();
                    if raw.is_empty() {
                        return;
                    }
                    let (from, to) = if let Some((from, to)) = raw.split_once('>') {
                        (from.trim().to_string(), to.trim().to_string())
                    } else if let Some(from) = current_account.clone() {
                        (from, raw)
                    } else {
                        return;
                    };
                    prompt_input.update(cx, |input, cx| {
                        input.set_value("", window, cx);
                    });
                    let controller = controller.clone();
                    runtime.spawn(async move {
                        let _ = controller.switch_local_account(&from, &to).await;
                    });
                })
        });

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

    let model_controls = snapshot.model_capabilities.iter().enumerate().fold(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(format!("Model: {active_model}")),
        |div, (index, capability)| {
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            let model = capability.model.clone();
            let mut button = Button::new(("workbench-model", index))
                .label(capability.model.clone())
                .on_click(move |_, _, _| {
                    let controller = controller.clone();
                    let model = model.clone();
                    runtime.spawn(async move {
                        let _ = controller.set_active_model(&model).await;
                    });
                });
            if active_model == capability.model {
                button = button.primary();
            }
            div.child(button)
        },
    );

    let reasoning_controls = match active_capability {
        Some(capability) => capability.reasoning_values.iter().enumerate().fold(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(format!("Reasoning Effort: {active_reasoning}")),
            |div, (index, value)| {
                let controller = view.controller.clone();
                let runtime = view.runtime.clone();
                let effort = value.reasoning_effort.clone();
                let mut button = Button::new(("workbench-reasoning", index))
                    .label(value.reasoning_effort.clone())
                    .on_click(move |_, _, _| {
                        let controller = controller.clone();
                        let effort = effort.clone();
                        runtime.spawn(async move {
                            let _ = controller.set_active_reasoning_effort(&effort).await;
                        });
                    });
                if active_reasoning == value.reasoning_effort {
                    button = button.primary();
                }
                div.child(button)
            },
        ),
        None => div()
            .flex()
            .flex_col()
            .gap_1()
            .child("Reasoning Effort: unavailable"),
    };

    let speed_control = match active_capability {
        Some(capability) if capability.supports_speed => {
            let controller = view.controller.clone();
            let runtime = view.runtime.clone();
            let speed_target = !speed_enabled;
            Button::new("speed-toggle")
                .label(if speed_enabled {
                    "Speed: on"
                } else {
                    "Speed: off"
                })
                .on_click(move |_, _, _| {
                    let controller = controller.clone();
                    runtime.spawn(async move {
                        let _ = controller.set_active_speed(speed_target).await;
                    });
                })
                .into_any_element()
        }
        _ => Button::new("speed-disabled")
            .label("Speed: unavailable")
            .into_any_element(),
    };

    let plan_control = {
        let controller = view.controller.clone();
        let runtime = view.runtime.clone();
        let plan_target = !plan_enabled;
        Button::new("plan-toggle")
            .label(if plan_enabled {
                "Plan Mode: on"
            } else {
                "Plan Mode: off"
            })
            .on_click(move |_, _, _| {
                let controller = controller.clone();
                runtime.spawn(async move {
                    let _ = controller.set_plan_mode(plan_target).await;
                });
            })
    };

    let choice_prompt = super::widgets::choice_prompt::render_choice_prompt(view, snapshot);

    div()
        .flex()
        .size_full()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_1_5()
                .child(session_list)
                .child(capabilities),
        )
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
                        .child(speed_control)
                        .child(plan_control)
                        .child(
                            Button::new("new-file")
                                .label("New File")
                                .on_click(cx.listener(WorkdeskMainView::on_new_file)),
                        )
                        .child({
                            let controller = view.controller.clone();
                            let runtime = view.runtime.clone();
                            let prompt_input = view.prompt_input.clone();
                            Button::new("send-prompt").label("Send Prompt").on_click(
                                move |_, window, cx| {
                                    let prompt = prompt_input.read(cx).value().to_string();
                                    let prompt = prompt.trim().to_string();
                                    if prompt.is_empty() {
                                        return;
                                    }
                                    prompt_input.update(cx, |input, cx| {
                                        input.set_value("", window, cx);
                                    });
                                    let controller = controller.clone();
                                    runtime.spawn(async move {
                                        let _ = controller.send_prompt(&prompt).await;
                                    });
                                },
                            )
                        }),
                )
                .child(Input::new(&view.prompt_input).w_full())
                .child(model_controls)
                .child(reasoning_controls)
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
                .child(super::runs::render_runs(view, cx, snapshot))
                .child(super::files::render_files(view, cx, snapshot)),
        )
}
