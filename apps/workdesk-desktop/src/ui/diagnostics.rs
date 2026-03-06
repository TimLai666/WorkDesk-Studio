use super::*;

pub(super) fn render_diagnostics(
    diagnostics: &[crate::controller::UiDiagnostic],
) -> impl IntoElement {
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
