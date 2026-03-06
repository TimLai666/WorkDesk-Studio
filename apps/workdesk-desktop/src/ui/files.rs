use super::*;

pub(super) fn render_files(
    view: &WorkdeskMainView,
    cx: &mut Context<WorkdeskMainView>,
    snapshot: &crate::controller::UiStateSnapshot,
) -> impl IntoElement {
    let entries =
        snapshot
            .workspace_entries
            .iter()
            .fold(div().flex().flex_col().gap_1(), |div, entry| {
                let prefix = if entry.is_dir { "[dir]" } else { "[file]" };
                div.child(format!("{prefix} {}", entry.path))
            });
    let search = snapshot.file_search_results.iter().fold(
        div().flex().flex_col().gap_1().child("Search"),
        |div, item| div.child(format!("{}:{} {}", item.path, item.line, item.preview)),
    );
    let diff = match &snapshot.diff_result {
        Some(diff) => diff.hunks.iter().take(20).fold(
            div().flex().flex_col().gap_1().child("Diff"),
            |div, line| {
                div.child(format!(
                    "{} {:?}/{:?} {}",
                    line.kind, line.left_line, line.right_line, line.text
                ))
            },
        ),
        None => div().child("Diff: (none)"),
    };
    let terminal = snapshot
        .terminal_session
        .as_ref()
        .map(|session| {
            format!(
                "Terminal {} [{}] exit={:?}\nstdout:\n{}\nstderr:\n{}",
                session.session_id,
                session.status,
                session.exit_code,
                session.stdout,
                session.stderr
            )
        })
        .unwrap_or_else(|| "Terminal: (none)".into());

    let _ = view;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("file-open-readme")
                        .label("Open README")
                        .on_click(cx.listener(WorkdeskMainView::on_file_open_readme)),
                )
                .child(
                    Button::new("file-save")
                        .label("Save")
                        .on_click(cx.listener(WorkdeskMainView::on_file_save)),
                )
                .child(
                    Button::new("file-search")
                        .label("Search TODO")
                        .on_click(cx.listener(WorkdeskMainView::on_file_search_todo)),
                )
                .child(
                    Button::new("file-diff")
                        .label("Self Diff")
                        .on_click(cx.listener(WorkdeskMainView::on_file_diff_self)),
                )
                .child(
                    Button::new("file-terminal")
                        .label("Terminal")
                        .on_click(cx.listener(WorkdeskMainView::on_file_terminal_dir)),
                ),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .w_1_2()
                        .child("Workspace")
                        .child(entries),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .w_1_2()
                        .child(format!(
                            "Editor: {}",
                            snapshot
                                .current_file_path
                                .clone()
                                .unwrap_or_else(|| "(none)".into())
                        ))
                        .child(snapshot.current_file_content.clone()),
                ),
        )
        .child(search)
        .child(diff)
        .child(terminal)
}
