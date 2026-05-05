use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct BotState {
    pub known_matches: HashMap<i64, MatchState>,
    pub last_daily_reminder: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatchState {
    pub id: i64,
    pub status: MatchStatus,
    pub notified_announced: bool,
    pub notified_live: bool,
    pub notified_result: bool,
    pub notified_result_24h: bool,
    pub name: String,
    pub scheduled_at: Option<String>,
    pub result_timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum MatchStatus {
    Upcoming,
    Running,
    Finished,
    Canceled,
}

impl MatchState {
    pub fn new_upcoming(match_data: &PsMatch) -> Self {
        Self {
            id: match_data.id,
            status: MatchStatus::Upcoming,
            notified_announced: false,
            notified_live: false,
            notified_result: false,
            notified_result_24h: false,
            name: match_data.name.clone().unwrap_or_default(),
            scheduled_at: match_data.scheduled_at.clone(),
            result_timestamp: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PsMatch {
    pub id: i64,
    pub name: Option<String>,
    pub status: String,
    pub match_type: Option<String>,
    pub number_of_games: Option<i32>,
    pub scheduled_at: Option<String>,
    pub begin_at: Option<String>,
    pub end_at: Option<String>,
    pub tournament: Option<PsTournament>,
    pub opponents: Vec<PsOpponentWrapper>,
    pub results: Option<Vec<PsResult>>,
    pub winner: Option<PsTeam>,
    pub streams_list: Option<Vec<PsStream>>,
}

#[derive(Debug, Deserialize)]
pub struct PsTournament {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PsOpponentWrapper {
    pub opponent: PsTeam,
}

#[derive(Debug, Deserialize)]
pub struct PsTeam {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PsResult {
    pub team_id: i64,
    pub score: i32,
}

#[derive(Debug, Deserialize)]
pub struct PsStream {
    pub language: Option<String>,
    pub raw_url: Option<String>,
}

impl BotState {
    pub fn new() -> Self {
        Self {
            known_matches: HashMap::new(),
            last_daily_reminder: None,
        }
    }

    pub async fn save_to_db(&self, pool: &SqlitePool) -> Result<()> {
        // Delete all existing records
        sqlx::query("DELETE FROM bot_state").execute(pool).await?;

        // Save the state
        let serialized = serde_json::to_string_pretty(self)?;
        sqlx::query("INSERT INTO bot_state (data) VALUES (?)")
            .bind(serialized)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn load_from_db(pool: &SqlitePool) -> Result<Self> {
        let row = sqlx::query("SELECT data FROM bot_state LIMIT 1")
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let data: String = row.get(0);
            let state: BotState = serde_json::from_str(&data)?;
            Ok(state)
        } else {
            Ok(Self::new())
        }
    }

    pub async fn init_db(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bot_state (
                id INTEGER PRIMARY KEY,
                data TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
