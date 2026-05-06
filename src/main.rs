mod notifications;
mod pandascore;
mod state;
mod telegram;

use anyhow::Result;
use chrono::{Timelike, Utc};
use chrono_tz::Europe::Paris;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

use crate::notifications::{
    format_announced, format_daily_reminder, format_live, format_result, format_result_24h,
    match_is_today,
};
use crate::pandascore::PandaScoreClient;
use crate::state::{BotState, MatchStatus};
use crate::telegram::TelegramClient;

#[derive(Debug, Clone)]
pub struct Config {
    pub pandascore_token: String,
    pub vitality_team_id: i64,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub poll_interval: u64,
    pub morning_hour: u32,
    pub port: u16,
    pub state_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            pandascore_token: env::var("PANDASCORE_TOKEN").expect("PANDASCORE_TOKEN must be set"),
            vitality_team_id: env::var("VITALITY_TEAM_ID")
                .unwrap_or_else(|_| "9565".to_string())
                .parse()
                .expect("VITALITY_TEAM_ID must be a number"),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .expect("TELEGRAM_BOT_TOKEN must be set"),
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID must be set"),
            poll_interval: env::var("POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "120".to_string())
                .parse()
                .expect("POLL_INTERVAL_SECS must be a number"),
            morning_hour: env::var("MORNING_REMINDER_HOUR")
                .unwrap_or_else(|_| "9".to_string())
                .parse()
                .expect("MORNING_REMINDER_HOUR must be a number"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            state_path: env::var("STATE_PATH").unwrap_or_else(|_| "state.db".to_string()),
        }
    }
}

async fn health_check_server(port: u16) -> Result<()> {
    use warp::Filter;

    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({"status": "healthy"})));

    let routes = health.with(warp::cors().allow_any_origin());

    tracing::info!("Health check server starting on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

async fn poll_cycle(
    ps: &PandaScoreClient,
    tg: &TelegramClient,
    bot_state: &mut BotState,
    pool: &sqlx::SqlitePool,
    config: &Config,
) -> Result<()> {
    tracing::info!("Checking for matches...");

    let upcoming = ps.fetch_upcoming().await.unwrap_or_else(|e| {
        tracing::error!("Failed to fetch upcoming matches: {}", e);
        vec![]
    });
    let running = ps.fetch_running().await.unwrap_or_else(|e| {
        tracing::error!("Failed to fetch running matches: {}", e);
        vec![]
    });
    let past = ps.fetch_past(10).await.unwrap_or_else(|e| {
        tracing::error!("Failed to fetch past matches: {}", e);
        vec![]
    });

    for m in &upcoming {
        let entry = bot_state
            .known_matches
            .entry(m.id)
            .or_insert_with(|| state::MatchState::new_upcoming(m));

        if !entry.notified_announced {
            let msg = format_announced(m);
            if let Err(e) = tg.send_message(&msg).await {
                tracing::error!("Failed to send announced notification for match {}: {}", m.id, e);
                continue;
            }
            entry.notified_announced = true;
            tracing::info!("Notified announced match {}", m.id);
        }
    }

    for m in &running {
        let entry = bot_state
            .known_matches
            .entry(m.id)
            .or_insert_with(|| state::MatchState::new_upcoming(m));

        entry.status = MatchStatus::Running;

        if !entry.notified_live {
            let msg = format_live(m);
            if let Err(e) = tg.send_message(&msg).await {
                tracing::error!("Failed to send live notification for match {}: {}", m.id, e);
                continue;
            }
            entry.notified_live = true;
            tracing::info!("Notified live match {}", m.id);
        }
    }

    for m in &past {
        let entry = bot_state
            .known_matches
            .entry(m.id)
            .or_insert_with(|| state::MatchState::new_upcoming(m));

        entry.status = MatchStatus::Finished;

        if !entry.notified_result {
            let msg = format_result(m, config.vitality_team_id);
            if let Err(e) = tg.send_message(&msg).await {
                tracing::error!("Failed to send result notification for match {}: {}", m.id, e);
                continue;
            }
            entry.notified_result = true;
            entry.result_timestamp = Some(Utc::now().to_rfc3339());
            tracing::info!("Notified result for match {}", m.id);
        }

        if !entry.notified_result_24h {
            if let Some(ref ts) = entry.result_timestamp {
                if let Ok(finished_at) = ts.parse::<chrono::DateTime<Utc>>() {
                    if Utc::now() - finished_at >= chrono::Duration::hours(24) {
                        let msg = format_result_24h(m, config.vitality_team_id);
                        if let Err(e) = tg.send_message(&msg).await {
                            tracing::error!("Failed to send 24h result notification for match {}: {}", m.id, e);
                            continue;
                        }
                        entry.notified_result_24h = true;
                        tracing::info!("Notified 24h result for match {}", m.id);
                    }
                }
            }
        }
    }

    let now_paris = Utc::now().with_timezone(&Paris);
    let today_str = now_paris.format("%Y-%m-%d").to_string();

    let already_sent = bot_state
        .last_daily_reminder
        .as_ref()
        .map_or(false, |d| d == &today_str);

    if !already_sent && now_paris.hour() >= config.morning_hour && now_paris.hour() < config.morning_hour + 1 {
        let today_matches: Vec<_> = upcoming.iter().filter(|m| match_is_today(m)).collect();
        let msg = format_daily_reminder(&today_matches);
        if let Err(e) = tg.send_message(&msg).await {
            tracing::error!("Failed to send daily reminder: {}", e);
        } else {
            bot_state.last_daily_reminder = Some(today_str);
            tracing::info!("Sent daily morning reminder");
        }
    }

    bot_state.save_to_db(pool).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env();
    tracing::info!("Vitality Bot starting");

    let db_url = format!("sqlite:{}?mode=rwc", config.state_path);
    let pool = sqlx::SqlitePool::connect(&db_url).await?;
    BotState::init_db(&pool).await?;

    let mut bot_state = BotState::load_from_db(&pool).await?;

    let ps = PandaScoreClient::new(config.pandascore_token.clone(), config.vitality_team_id);
    let tg = TelegramClient::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id.clone(),
    );

    let health_port = config.port;
    tokio::spawn(async move {
        if let Err(e) = health_check_server(health_port).await {
            tracing::error!("Health check server failed: {}", e);
        }
    });

    tracing::info!("Starting main polling loop (interval: {}s)", config.poll_interval);

    loop {
        match poll_cycle(&ps, &tg, &mut bot_state, &pool, &config).await {
            Ok(_) => tracing::info!("Poll cycle completed"),
            Err(e) => tracing::error!("Poll cycle error: {}", e),
        }
        sleep(Duration::from_secs(config.poll_interval)).await;
    }
}
