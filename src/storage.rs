use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::email::EmailInfo;
use crate::models::EmailManifest;

pub struct LocalStorage {
    base_dir: PathBuf,
}

impl LocalStorage {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn email_dir(&self, info: &EmailInfo) -> PathBuf {
        self.base_dir
            .join(info.date_folder())
            .join(info.dir_name())
    }

    pub async fn ensure_email_dir(&self, info: &EmailInfo) -> Result<PathBuf> {
        let dir = self.email_dir(info);
        fs::create_dir_all(&dir)
            .await
            .context("Failed to create email storage directory")?;
        Ok(dir)
    }

    pub async fn store_image(
        &self,
        dir: &Path,
        data: &[u8],
        filename: &str,
    ) -> Result<String> {
        let path = dir.join(filename);

        fs::write(&path, data)
            .await
            .context("Failed to write image file")?;

        tracing::info!(path = %path.display(), "Stored image");
        Ok(filename.to_string())
    }

    pub async fn store_manifest(&self, dir: &Path, manifest: &EmailManifest) -> Result<()> {
        let path = dir.join("manifest.json");
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(&path, json)
            .await
            .context("Failed to write manifest")?;

        tracing::info!(path = %path.display(), "Stored manifest");
        Ok(())
    }
}
