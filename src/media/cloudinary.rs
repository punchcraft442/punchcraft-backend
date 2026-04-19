use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::multipart;
use serde::Deserialize;
use sha1::{Digest, Sha1};

use crate::common::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct CloudinaryUploadResponse {
    pub secure_url: String,
    pub public_id: String,
}

pub struct CloudinaryClient {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

impl CloudinaryClient {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            cloud_name: std::env::var("CLOUDINARY_CLOUD_NAME")
                .map_err(|_| AppError::Internal(anyhow::anyhow!("CLOUDINARY_CLOUD_NAME is not set")))?,
            api_key: std::env::var("CLOUDINARY_API_KEY")
                .map_err(|_| AppError::Internal(anyhow::anyhow!("CLOUDINARY_API_KEY is not set")))?,
            api_secret: std::env::var("CLOUDINARY_API_SECRET")
                .map_err(|_| AppError::Internal(anyhow::anyhow!("CLOUDINARY_API_SECRET is not set")))?,
        })
    }

    fn sign(&self, params_to_sign: &str) -> String {
        let to_sign = format!("{}{}", params_to_sign, self.api_secret);
        let mut hasher = Sha1::new();
        hasher.update(to_sign.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn timestamp() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    pub async fn upload(
        &self,
        data: Vec<u8>,
        filename: String,
        folder: &str,
    ) -> Result<CloudinaryUploadResponse, AppError> {
        self.upload_resource(data, filename, folder, "image").await
    }

    pub async fn upload_auto(
        &self,
        data: Vec<u8>,
        filename: String,
        folder: &str,
    ) -> Result<CloudinaryUploadResponse, AppError> {
        self.upload_resource(data, filename, folder, "auto").await
    }

    async fn upload_resource(
        &self,
        data: Vec<u8>,
        filename: String,
        folder: &str,
        resource_type: &str,
    ) -> Result<CloudinaryUploadResponse, AppError> {
        if data.is_empty() {
            return Err(AppError::BadRequest("Uploaded file is empty".into()));
        }

        let ext = filename.rsplit('.').next().unwrap_or("bin");
        let safe_filename = format!(
            "upload.{}",
            ext.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>()
        );

        tracing::debug!(
            "Cloudinary upload: {} bytes, filename={:?} -> {:?}, folder={}, resource_type={}",
            data.len(), filename, safe_filename, folder, resource_type
        );

        let timestamp = Self::timestamp();
        let params_to_sign = format!("folder={}&timestamp={}", folder, timestamp);
        let signature = self.sign(&params_to_sign);

        let url = format!(
            "https://api.cloudinary.com/v1_1/{}/{}/upload",
            self.cloud_name, resource_type
        );

        let part = multipart::Part::stream(reqwest::Body::from(data))
            .file_name(safe_filename)
            .mime_str("application/octet-stream")
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let form = multipart::Form::new()
            .text("api_key", self.api_key.clone())
            .text("timestamp", timestamp)
            .text("signature", signature)
            .text("folder", folder.to_string())
            .part("file", part);

        let resp = reqwest::Client::new()
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("Upload failed: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!("Cloudinary error: {body}")));
        }

        resp.json::<CloudinaryUploadResponse>()
            .await
            .map_err(|e| AppError::BadRequest(format!("Cloudinary response error: {e}")))
    }

    pub async fn delete(&self, public_id: &str) -> Result<(), AppError> {
        let timestamp = Self::timestamp();
        let params_to_sign = format!("public_id={}&timestamp={}", public_id, timestamp);
        let signature = self.sign(&params_to_sign);

        let url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/destroy",
            self.cloud_name
        );

        reqwest::Client::new()
            .post(&url)
            .form(&[
                ("api_key", self.api_key.as_str()),
                ("timestamp", timestamp.as_str()),
                ("signature", signature.as_str()),
                ("public_id", public_id),
            ])
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("Delete failed: {e}")))?;

        Ok(())
    }
}
