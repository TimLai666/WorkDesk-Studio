use super::super::*;

pub(crate) fn render_choice_prompt(
    view: &WorkdeskMainView,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    match &snapshot.pending_choice_prompt {
        Some(prompt) => {
            let options = prompt.options.iter().enumerate().fold(
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
                                    .answer_choice_prompt_option(
                                        &session_id,
                                        &prompt_id,
                                        &option_id,
                                    )
                                    .await;
                            });
                        });
                    if prompt.recommended_option_id.as_deref() == Some(option.option_id.as_str()) {
                        button = button.primary();
                    }
                    div.child(button)
                },
            );

            if prompt.allow_freeform {
                options
                    .child("Use composer text for freeform answer.")
                    .child({
                        let controller = view.controller.clone();
                        let runtime = view.runtime.clone();
                        let prompt_input = view.prompt_input.clone();
                        let session_id = prompt.session_id.clone();
                        let prompt_id = prompt.prompt_id.clone();
                        Button::new("prompt-freeform")
                            .label("Submit Freeform")
                            .on_click(move |_, window, cx| {
                                let text = prompt_input.read(cx).value().to_string();
                                let text = text.trim().to_string();
                                if text.is_empty() {
                                    return;
                                }
                                prompt_input.update(cx, |input, cx| {
                                    input.set_value("", window, cx);
                                });
                                let controller = controller.clone();
                                let session_id = session_id.clone();
                                let prompt_id = prompt_id.clone();
                                runtime.spawn(async move {
                                    let _ = controller
                                        .answer_choice_prompt_text(&session_id, &prompt_id, &text)
                                        .await;
                                });
                            })
                    })
                    .into_any_element()
            } else {
                options.into_any_element()
            }
        }
        .into_any_element(),
        None => div().child("Choice Prompt: (none)").into_any_element(),
    }
}
