use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn copy_path_recursive(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let source = source.clone();
    let target = target.clone();
    tokio::task::spawn_blocking(move || copy_path_recursive_sync(&source, &target))
        .await
        .context("join skill copy task")??;
    Ok(())
}

fn copy_path_recursive_sync(source: &PathBuf, target: &PathBuf) -> Result<()> {
    let metadata = std::fs::metadata(source)
        .with_context(|| format!("skill source path not found: {}", source.display()))?;
    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, target)?;
        return Ok(());
    }

    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let nested_target = target.join(entry.file_name());
        copy_path_recursive_sync(&path, &nested_target)?;
    }
    Ok(())
}
