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
    pub fn from_env() -> Self {
        Self {
            cloud_name: std::env::var("CLOUDINARY_CLOUD_NAME")
                .expect("CLOUDINARY_CLOUD_NAME must be set"),
            api_key: std::env::var("CLOUDINARY_API_KEY")
                .expect("CLOUDINARY_API_KEY must be set"),
            api_secret: std::env::var("CLOUDINARY_API_SECRET")
                .expect("CLOUDINARY_API_SECRET must be set"),
        }
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
        let timestamp = Self::timestamp();
        let params_to_sign = format!("folder={}&timestamp={}", folder, timestamp);
        let signature = self.sign(&params_to_sign);

        let url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/upload",
            self.cloud_name
        );

        let part = multipart::Part::bytes(data)
            .file_name(filename)
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
