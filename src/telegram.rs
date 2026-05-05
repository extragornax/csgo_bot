use anyhow::Result;
use reqwest;
use serde_json::json;

#[derive(Debug)]
pub struct TelegramClient {
    http: reqwest::Client,
    token: String,
    chat_id: String,
}

impl TelegramClient {
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build reqwest client"),
            token,
            chat_id,
        }
    }

    pub async fn send_message(&self, text: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let body = json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true
        });

        let resp = self.http.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("telegram error: {err}");
        }
        Ok(())
    }
}
