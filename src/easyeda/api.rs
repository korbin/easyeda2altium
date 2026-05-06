//! EasyEDA API client. Async via reqwest. Optional disk cache.

use std::path::{Path, PathBuf};

use reqwest::Client;
use serde_json::Value;

use crate::error::{Error, Result};

const API_ENDPOINT: &str = "https://easyeda.com/api/products/{lcsc_id}/components";
const ENDPOINT_3D_OBJ: &str = "https://modules.easyeda.com/3dmodel/{uuid}";
const ENDPOINT_3D_STEP: &str = "https://modules.easyeda.com/qAxj6KHrDKw4blvCG8QJPs7Y/{uuid}";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Clone)]
pub struct EasyedaApi {
    client: Client,
    cache_dir: Option<PathBuf>,
}

impl EasyedaApi {
    pub fn new(use_cache: bool) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .gzip(true)
            .build()?;
        let cache_dir = if use_cache {
            let dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".easyeda_cache");
            std::fs::create_dir_all(&dir).ok();
            Some(dir)
        } else {
            None
        };
        Ok(Self { client, cache_dir })
    }

    fn cache_path(&self, identifier: &str, ext: &str) -> Option<PathBuf> {
        let dir = self.cache_dir.as_ref()?;
        let safe = identifier.replace(['/', '\\'], "_");
        Some(dir.join(format!("{}.{}", safe, ext)))
    }

    fn read_text_cache(&self, p: &Path) -> Option<String> {
        std::fs::read_to_string(p).ok()
    }

    fn read_bytes_cache(&self, p: &Path) -> Option<Vec<u8>> {
        std::fs::read(p).ok()
    }

    fn write_text_cache(p: &Path, contents: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let _ = std::fs::write(p, contents);
    }

    fn write_bytes_cache(p: &Path, bytes: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let _ = std::fs::write(p, bytes);
    }

    /// Fetch component CAD data. Returns the `result` field from the API
    /// envelope (after success-check).
    pub async fn get_cad_data(&self, lcsc_id: &str) -> Result<Value> {
        if let Some(p) = self.cache_path(lcsc_id, "json") {
            if let Some(text) = self.read_text_cache(&p) {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    return Self::extract_result(v);
                }
            }
        }

        let url = API_ENDPOINT.replace("{lcsc_id}", lcsc_id);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Api(format!("status {} for {}", status, url)));
        }
        let text = resp.text().await?;
        if let Some(p) = self.cache_path(lcsc_id, "json") {
            Self::write_text_cache(&p, &text);
        }
        let v: Value = serde_json::from_str(&text)?;
        Self::extract_result(v)
    }

    fn extract_result(v: Value) -> Result<Value> {
        if v.get("success") == Some(&Value::Bool(false)) {
            return Err(Error::Api(format!(
                "API returned success=false: {}",
                v.get("message")
                    .map(|m| m.to_string())
                    .unwrap_or_default()
            )));
        }
        match v.get("result") {
            Some(r) => Ok(r.clone()),
            None => Err(Error::Api("missing 'result' in API response".into())),
        }
    }

    /// Fetch raw STEP bytes for a 3D-model uuid. Returns None on 404.
    pub async fn get_step(&self, uuid: &str) -> Result<Option<Vec<u8>>> {
        if let Some(p) = self.cache_path(uuid, "step") {
            if let Some(b) = self.read_bytes_cache(&p) {
                return Ok(Some(b));
            }
        }
        let url = ENDPOINT_3D_STEP.replace("{uuid}", uuid);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let bytes = resp.bytes().await?.to_vec();
        if let Some(p) = self.cache_path(uuid, "step") {
            Self::write_bytes_cache(&p, &bytes);
        }
        Ok(Some(bytes))
    }

    pub async fn get_obj(&self, uuid: &str) -> Result<Option<String>> {
        if let Some(p) = self.cache_path(uuid, "obj") {
            if let Some(t) = self.read_text_cache(&p) {
                return Ok(Some(t));
            }
        }
        let url = ENDPOINT_3D_OBJ.replace("{uuid}", uuid);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let text = resp.text().await?;
        if let Some(p) = self.cache_path(uuid, "obj") {
            Self::write_text_cache(&p, &text);
        }
        Ok(Some(text))
    }
}
