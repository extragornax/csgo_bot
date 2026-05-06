use anyhow::Result;
use std::time::Duration;

use crate::state::PsMatch;

#[derive(Debug)]
pub struct PandaScoreClient {
    http: reqwest::Client,
    token: String,
    team_id: i64,
}

impl PandaScoreClient {
    pub fn new(token: String, team_id: i64) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("vitality-bot/1.0")
                .build()
                .expect("Failed to build reqwest client"),
            token,
            team_id,
        }
    }

    async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Vec<PsMatch>> {
        let url = format!("https://api.pandascore.co{}", path);
        let team_id = self.team_id.to_string();

        let mut all_params = vec![("filter[opponent_id]", team_id.as_str())];
        all_params.extend_from_slice(params);

        let response = self
            .http
            .get(&url)
            .query(&all_params)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("PandaScore API error on {}: {} — {}", url, status, body);
        }

        let matches: Vec<PsMatch> = response.json().await?;
        Ok(matches)
    }

    pub async fn fetch_upcoming(&self) -> Result<Vec<PsMatch>> {
        self.get("/csgo/matches/upcoming", &[("sort", "begin_at"), ("per_page", "50")])
            .await
    }

    pub async fn fetch_running(&self) -> Result<Vec<PsMatch>> {
        self.get("/csgo/matches/running", &[]).await
    }

    pub async fn fetch_past(&self, count: usize) -> Result<Vec<PsMatch>> {
        let per_page = count.to_string();
        self.get("/csgo/matches/past", &[("sort", "-end_at"), ("per_page", &per_page)])
            .await
    }
}
