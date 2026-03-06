use super::*;

pub(super) fn render_office(
    view: &mut WorkdeskMainView,
    window: &mut Window,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let versions = snapshot.office_versions.iter().fold(
        div().flex().flex_col().gap_1().child("Versions"),
        |div, version| div.child(version.clone()),
    );
    let pdf_op = snapshot
        .pdf_last_operation
        .as_ref()
        .map(|operation| {
            format!(
                "PDF op: path={} replaced={} version={}",
                operation.path, operation.replaced_count, operation.version_name
            )
        })
        .unwrap_or_else(|| "PDF op: (none)".into());

    let embed_panel = if let Some(embed_url) = snapshot.office_embed_url.as_deref() {
        #[cfg(feature = "webview")]
        {
            match view.ensure_office_webview(window, cx, embed_url) {
                Some(webview) => div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child("OnlyOffice (embedded)")
                    .child(div().w_full().h(px(420.0)).child(webview))
                    .child(format!("Embed URL: {embed_url}"))
                    .into_any_element(),
                None => div()
                    .child("OnlyOffice embed failed to initialize")
                    .child(format!("Embed URL: {embed_url}"))
                    .into_any_element(),
            }
        }
        #[cfg(not(feature = "webview"))]
        {
            div()
                .child("OnlyOffice embed unavailable (webview feature disabled)")
                .child(format!("Embed URL: {embed_url}"))
                .into_any_element()
        }
    } else {
        view.hide_office_webview(cx);
        div()
            .child("OnlyOffice embed: inactive (open DOCX/XLSX/PPTX to activate)")
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("office-open")
                        .label("Open README")
                        .on_click(cx.listener(WorkdeskMainView::on_office_open)),
                )
                .child(
                    Button::new("office-save")
                        .label("Save Office")
                        .on_click(cx.listener(WorkdeskMainView::on_office_save)),
                )
                .child(
                    Button::new("pdf-preview")
                        .label("Preview PDF")
                        .on_click(cx.listener(WorkdeskMainView::on_pdf_preview)),
                )
                .child(
                    Button::new("pdf-annotate")
                        .label("Annotate")
                        .on_click(cx.listener(WorkdeskMainView::on_pdf_annotate)),
                )
                .child(
                    Button::new("pdf-replace")
                        .label("Replace")
                        .on_click(cx.listener(WorkdeskMainView::on_pdf_replace)),
                )
                .child(
                    Button::new("pdf-version")
                        .label("Save Version")
                        .on_click(cx.listener(WorkdeskMainView::on_pdf_save_version)),
                ),
        )
        .child(format!(
            "Document: {}",
            snapshot
                .office_path
                .clone()
                .unwrap_or_else(|| "(none)".into())
        ))
        .child(embed_panel)
        .child(snapshot.office_editor_text.clone())
        .child(versions)
        .child(pdf_op)
}
