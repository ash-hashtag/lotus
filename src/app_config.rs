use std::path::Path;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AppConfig {
    font_size: u16,
}

impl AppConfig {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub async fn write_json_file(&self, path: &Path) -> anyhow::Result<()> {
        tokio::fs::write(path, serde_json::to_string(&self)?.as_bytes()).await?;
        Ok(())
    }
}
