use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::state::{
    MatchState, MatchStatus, PsMatch, PsOpponentWrapper, PsResult, PsStream, PsTeam, PsTournament,
};

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

    pub async fn fetch_upcoming(&self) -> Result<Vec<PsMatch>> {
        let url = format!(
            "https://api.pandascore.co/csgo/matches/upcoming?filter[opponent_id]={}&sort=begin_at&per_page=50",
            self.team_id
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("PandaScore API error: {}", response.status());
        }

        let matches: Vec<PsMatch> = response.json().await?;
        Ok(matches)
    }

    pub async fn fetch_running(&self) -> Result<Vec<PsMatch>> {
        let url = format!(
            "https://api.pandascore.co/csgo/matches/running?filter[opponent_id]={}",
            self.team_id
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("PandaScore API error: {}", response.status());
        }

        let matches: Vec<PsMatch> = response.json().await?;
        Ok(matches)
    }

    pub async fn fetch_past(&self, count: usize) -> Result<Vec<PsMatch>> {
        let url = format!(
            "https://api.pandascore.co/csgo/matches/past?filter[opponent_id]={}&sort=-end_at&per_page={}",
            self.team_id, count
        );

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("PandaScore API error: {}", response.status());
        }

        let matches: Vec<PsMatch> = response.json().await?;
        Ok(matches)
    }
}
