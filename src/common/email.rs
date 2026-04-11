use reqwest::Client;
use serde_json::json;
use tracing::error;

const RESEND_URL: &str = "https://api.resend.com/emails";

#[derive(Clone)]
pub struct EmailService {
    client: Client,
    api_key: String,
    from: String,
    admin_email: String,
}

impl EmailService {
    pub fn new(api_key: String, from: String, admin_email: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            from,
            admin_email,
        }
    }

    async fn send(&self, to: &str, subject: &str, html: String) {
        let payload = json!({
            "from": self.from,
            "to": [to],
            "subject": subject,
            "html": html,
        });

        let result = self
            .client
            .post(RESEND_URL)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await;

        match result {
            Ok(resp) if !resp.status().is_success() => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                error!("Resend rejected email to {to}: {status} — {body}");
            }
            Err(e) => error!("Failed to send email to {to}: {e}"),
            _ => {}
        }
    }

    /// Fires an activation email after a user registers. Non-blocking.
    pub fn send_activation_email(&self, to: String, activation_token: String) {
        let svc = self.clone();
        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        tokio::spawn(async move {
            let link = format!("{frontend_url}/verify-email?token={activation_token}");
            svc.send(
                &to,
                "Activate Your PunchCraft Account",
                format!(
                    "<h2>Welcome to PunchCraft!</h2>\
                     <p>Please click the link below to activate your account. \
                     This link expires in <strong>7 days</strong>.</p>\
                     <p><a href=\"{link}\">Activate Account</a></p>\
                     <p>If you did not create this account, you can safely ignore this email.</p>"
                ),
            )
            .await;
        });
    }

    /// Fires a welcome email after a user registers. Non-blocking.
    pub fn send_welcome(&self, to: String, name: String) {
        let svc = self.clone();
        tokio::spawn(async move {
            svc.send(
                &to,
                "Welcome to PunchCraft",
                format!(
                    "<h2>Welcome, {name}!</h2>\
                     <p>Your PunchCraft account has been created.</p>\
                     <p>Create your profile and submit it for review to get listed in the directory.</p>"
                ),
            )
            .await;
        });
    }

    /// Fires to the admin when a user submits a profile for review. Non-blocking.
    pub fn send_profile_submitted(&self, display_name: String, profile_id: String, role: String) {
        let svc = self.clone();
        let admin = svc.admin_email.clone();
        tokio::spawn(async move {
            svc.send(
                &admin,
                "New Profile Pending Review",
                format!(
                    "<h2>Profile Submitted for Review</h2>\
                     <p><strong>{display_name}</strong> ({role}) has submitted their profile.</p>\
                     <p>Profile ID: <code>{profile_id}</code></p>\
                     <p>Please log in to the admin panel to review it.</p>"
                ),
            )
            .await;
        });
    }

    /// Fires to the user when their profile is approved. Non-blocking.
    pub fn send_profile_approved(&self, to: String, display_name: String) {
        let svc = self.clone();
        tokio::spawn(async move {
            svc.send(
                &to,
                "Your PunchCraft Profile Has Been Approved",
                format!(
                    "<h2>Profile Approved</h2>\
                     <p>Hi {display_name},</p>\
                     <p>Your profile has been reviewed and approved. \
                     You are now listed in the PunchCraft directory.</p>"
                ),
            )
            .await;
        });
    }

    /// Fires a password reset email with a tokenised link. Non-blocking.
    pub fn send_password_reset(&self, to: String, reset_token: String) {
        let svc = self.clone();
        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        tokio::spawn(async move {
            let reset_link = format!("{frontend_url}/reset-password?token={reset_token}");
            svc.send(
                &to,
                "Reset Your PunchCraft Password",
                format!(
                    "<h2>Password Reset Request</h2>\
                     <p>Click the link below to reset your password. This link expires in <strong>1 hour</strong>.</p>\
                     <p><a href=\"{reset_link}\">Reset Password</a></p>\
                     <p>If you did not request a password reset, you can safely ignore this email.</p>"
                ),
            )
            .await;
        });
    }

    /// Fires to the user when their profile is rejected. Non-blocking.
    pub fn send_profile_rejected(&self, to: String, display_name: String, reason: String) {
        let svc = self.clone();
        tokio::spawn(async move {
            svc.send(
                &to,
                "Your PunchCraft Profile Needs Attention",
                format!(
                    "<h2>Profile Not Approved</h2>\
                     <p>Hi {display_name},</p>\
                     <p>Your profile could not be approved for the following reason:</p>\
                     <blockquote>{reason}</blockquote>\
                     <p>Please update your profile and resubmit for review.</p>"
                ),
            )
            .await;
        });
    }
}
