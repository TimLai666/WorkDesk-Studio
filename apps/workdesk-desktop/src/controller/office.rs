use super::*;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use urlencoding::encode;

impl DesktopAppController {
    pub async fn open_office_document(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::OfficeDesk));
        let response = self.api.office_open(path).await?;
        let versions = self
            .api
            .office_versions(path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let raw = STANDARD.decode(response.content_base64.as_bytes())?;
        let editor_text = String::from_utf8_lossy(&raw).to_string();
        let embed_url = build_office_embed_url(path);
        self.apply(ControllerAction::SetOffice {
            path: Some(path.to_string()),
            content_base64: Some(response.content_base64),
            editor_text,
            embed_url,
            versions,
            pdf_last_operation: None,
        });
        Ok(())
    }

    pub fn set_office_editor_text(&self, text: String) {
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: snapshot.office_path,
            content_base64: snapshot.office_content_base64,
            editor_text: text,
            embed_url: snapshot.office_embed_url,
            versions: snapshot.office_versions,
            pdf_last_operation: snapshot.pdf_last_operation,
        });
    }

    pub async fn save_office_document(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let path = snapshot
            .office_path
            .clone()
            .ok_or_else(|| anyhow!("no office document selected"))?;
        let base64_content = STANDARD.encode(snapshot.office_editor_text.as_bytes());
        self.api.office_save(&path, base64_content.clone()).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: Some(base64_content),
            editor_text: snapshot.office_editor_text,
            embed_url: snapshot.office_embed_url,
            versions,
            pdf_last_operation: snapshot.pdf_last_operation,
        });
        Ok(())
    }

    pub async fn preview_pdf(&self, path: &str) -> Result<()> {
        self.apply(ControllerAction::SetRoute(UiRoute::OfficeDesk));
        let preview = self.api.pdf_preview(path).await?;
        let versions = self
            .api
            .office_versions(path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let raw = STANDARD.decode(preview.content_base64.as_bytes())?;
        self.apply(ControllerAction::SetOffice {
            path: Some(path.to_string()),
            content_base64: Some(preview.content_base64),
            editor_text: String::from_utf8_lossy(&raw).to_string(),
            embed_url: None,
            versions,
            pdf_last_operation: None,
        });
        Ok(())
    }

    pub async fn annotate_pdf(&self, annotation: &str) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_annotate(&path, annotation).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            embed_url: snapshot.office_embed_url,
            versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }

    pub async fn replace_pdf_text(&self, search: &str, replace: &str) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_replace_text(&path, search, replace).await?;
        self.preview_pdf(&path).await?;
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            embed_url: None,
            versions: snapshot.office_versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }

    pub async fn save_pdf_version(&self) -> Result<()> {
        let path = self
            .snapshot()
            .office_path
            .ok_or_else(|| anyhow!("no PDF selected"))?;
        let operation = self.api.pdf_save_version(&path).await?;
        let versions = self
            .api
            .office_versions(&path)
            .await
            .map(|payload| payload.versions)
            .unwrap_or_default();
        let snapshot = self.snapshot();
        self.apply(ControllerAction::SetOffice {
            path: Some(path),
            content_base64: snapshot.office_content_base64,
            editor_text: snapshot.office_editor_text,
            embed_url: None,
            versions,
            pdf_last_operation: Some(operation),
        });
        Ok(())
    }
}

fn build_office_embed_url(path: &str) -> Option<String> {
    if !is_onlyoffice_editable(path) {
        return None;
    }

    let host = std::env::var("WORKDESK_ONLYOFFICE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("WORKDESK_ONLYOFFICE_PORT").unwrap_or_else(|_| "8044".into());
    let callback_base =
        std::env::var("WORKDESK_CORE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into());
    let callback_url = format!("{callback_base}/api/v1/office/onlyoffice/callback");
    let encoded_path = encode(path);
    let encoded_callback = encode(&callback_url);

    let template = std::env::var("WORKDESK_ONLYOFFICE_EMBED_TEMPLATE").unwrap_or_else(|_| {
        "http://{host}:{port}/?filePath={path}&callbackUrl={callback_url}".into()
    });
    Some(
        template
            .replace("{host}", &host)
            .replace("{port}", &port)
            .replace("{path}", encoded_path.as_ref())
            .replace("{callback_url}", encoded_callback.as_ref()),
    )
}

fn is_onlyoffice_editable(path: &str) -> bool {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|value| value.to_ascii_lowercase());
    matches!(extension.as_deref(), Some("docx" | "xlsx" | "pptx"))
}
