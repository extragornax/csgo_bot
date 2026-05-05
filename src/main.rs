use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::{DateTime, Utc, Datelike, Timelike};
use chrono_tz::Tz;
use tracing_subscriber;

#[derive(Debug, Clone)]
pub struct Config {
    pub pandascore_token: String,
    pub vitality_team_id: i64,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub poll_interval: u64,
    pub morning_hour: u32,
    pub timezone: String,
    pub port: u16,
    pub state_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            pandascore_token: env::var("PANDASCORE_TOKEN").expect("PANDASCORE_TOKEN must be set"),
            vitality_team_id: env::var("VITALITY_TEAM_ID")
                .expect("VITALITY_TEAM_ID must be set")
                .parse()
                .expect("VITALITY_TEAM_ID must be a number"),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set"),
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID must be set"),
            poll_interval: env::var("POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "120".to_string())
                .parse()
                .expect("POLL_INTERVAL_SECS must be a number"),
            morning_hour: env::var("MORNING_REMINDER_HOUR")
                .unwrap_or_else(|_| "9".to_string())
                .parse()
                .expect("MORNING_REMINDER_HOUR must be a number"),
            timezone: env::var("TIMEZONE").unwrap_or_else(|_| "Europe/Paris".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            state_path: env::var("STATE_PATH").unwrap_or_else(|_| "state.db".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Team {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Player {
    id: i64,
    name: String,
    nickname: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Match {
    id: i64,
    status: String,
    winner_id: Option<i64>,
    game: String,
    begin_at: Option<String>,
    end_at: Option<String>,
    teams: Vec<Team>,
    players: Vec<Player>,
    tournament: Tournament,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tournament {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse<T> {
    data: Vec<T>,
    pagination: Pagination,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pagination {
    page: i32,
    per_page: i32,
    total: i32,
    total_pages: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MatchState {
    id: i64,
    match_id: i64,
    status: String,
    notified: bool,
    notified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

// Database schema for tracking match states and notifications
const DB_INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS matches (
    id INTEGER PRIMARY KEY,
    match_id INTEGER UNIQUE NOT NULL,
    status TEXT NOT NULL,
    notified BOOLEAN DEFAULT FALSE,
    notified_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL,
    notification_type TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (match_id) REFERENCES matches (match_id)
);

CREATE INDEX IF NOT EXISTS idx_matches_match_id ON matches(match_id);
CREATE INDEX IF NOT EXISTS idx_matches_status ON matches(status);
CREATE INDEX IF NOT EXISTS idx_notifications_match_id ON notifications(match_id);
"#;

async fn initialize_database(pool: &SqlitePool) -> Result<()> {
    sqlx::query(DB_INIT_SQL).execute(pool).await?;
    Ok(())
}

async fn get_latest_matches(
    client: &reqwest::Client,
    config: &Config,
    team_id: i64,
) -> Result<Vec<Match>> {
    let url = format!(
        "https://api.pandascore.co/matches?filter[team_id]={}&page[size]=10&sort=-begin_at",
        team_id
    );
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.pandascore_token))
        .send()
        .await?;

    let api_response: ApiResponse<Match> = response.json().await?;
    Ok(api_response.data)
}

async fn get_match_details(
    client: &reqwest::Client,
    config: &Config,
    match_id: i64,
) -> Result<Option<Match>> {
    let url = format!("https://api.pandascore.co/matches/{}", match_id);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.pandascore_token))
        .send()
        .await?;

    if response.status().is_success() {
        let match_data: Match = response.json().await?;
        Ok(Some(match_data))
    } else {
        Ok(None)
    }
}

async fn send_telegram_message(
    client: &reqwest::Client,
    config: &Config,
    message: &str,
) -> Result<()> {
    use serde_json::json;

    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.telegram_bot_token
    );
    
    let request_body = json!({
        "chat_id": config.telegram_chat_id,
        "text": message,
        "parse_mode": "Markdown"
    });

    let response = client.post(&url).json(&request_body).send().await?;
    
    if response.status().is_success() {
        tracing::info!("Telegram message sent successfully");
    } else {
        tracing::error!("Failed to send Telegram message: {}", response.status());
    }
    
    Ok(())
}

async fn save_match_state(
    pool: &SqlitePool,
    match_state: &MatchState,
) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO matches (id, match_id, status, notified, notified_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )
    .bind(match_state.id)
    .bind(match_state.match_id)
    .bind(&match_state.status)
    .bind(match_state.notified)
    .bind(match_state.notified_at)
    .bind(match_state.created_at)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn get_match_state(
    pool: &SqlitePool,
    match_id: i64,
) -> Result<Option<MatchState>> {
    let row = sqlx::query(
        "SELECT id, match_id, status, notified, notified_at, created_at 
         FROM matches WHERE match_id = ?1"
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        Ok(Some(MatchState {
            id: row.get(0),
            match_id: row.get(1),
            status: row.get(2),
            notified: row.get(3),
            notified_at: row.get(4),
            created_at: row.get(5),
        }))
    } else {
        Ok(None)
    }
}

async fn record_notification(
    pool: &SqlitePool,
    match_id: i64,
    notification_type: &str,
    message: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO notifications (match_id, notification_type, message) 
         VALUES (?1, ?2, ?3)"
    )
    .bind(match_id)
    .bind(notification_type)
    .bind(message)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn check_and_notify_new_matches(
    client: &reqwest::Client,
    pool: &SqlitePool,
    config: &Config,
) -> Result<()> {
    tracing::info!("Checking for new matches...");
    
    let matches = get_latest_matches(client, config, config.vitality_team_id).await?;
    
    for match_data in matches {
        let match_id = match_data.id;
        
        // Check if we've seen this match before
        let existing_state = get_match_state(pool, match_id).await?;
        
        if let Some(state) = existing_state {
            // Match exists, check if status changed
            if state.status != match_data.status {
                tracing::info!("Match {} status changed from '{}' to '{}'", match_id, state.status, match_data.status);
                
                let message = format!(
                    "🏆 **Team Vitality Match Update**\n\n\
                    Match ID: {}\n\
                    Status: {}\n\
                    Tournament: {}\n\
                    Teams: {} vs {}\n\
                    Time: {}\n\
                    ",
                    match_id,
                    match_data.status,
                    match_data.tournament.name,
                    match_data.teams[0].name,
                    match_data.teams[1].name,
                    match_data.begin_at.as_ref().unwrap_or(&"Unknown".to_string())
                );
                
                // Send notification only if not already notified for this status change
                if !state.notified {
                    send_telegram_message(client, config, &message).await?;
                    
                    // Update state to mark as notified
                    let updated_state = MatchState {
                        id: state.id,
                        match_id,
                        status: match_data.status.clone(),
                        notified: true,
                        notified_at: Some(Utc::now()),
                        created_at: state.created_at,
                    };
                    
                    save_match_state(pool, &updated_state).await?;
                    
                    // Record the notification
                    record_notification(pool, match_id, "status_change", &message).await?;
                } else {
                    tracing::info!("Match {} already notified, skipping", match_id);
                }
            }
        } else {
            // New match, save initial state and notify
            tracing::info!("New match detected: {}", match_id);
            
            let message = format!(
                "🎮 **New Team Vitality Match**\n\n\
                Match ID: {}\n\
                Status: {}\n\
                Tournament: {}\n\
                Teams: {} vs {}\n\
                Time: {}\n\
                ",
                match_id,
                match_data.status,
                match_data.tournament.name,
                match_data.teams[0].name,
                match_data.teams[1].name,
                match_data.begin_at.as_ref().unwrap_or(&"Unknown".to_string())
            );
            
            send_telegram_message(client, config, &message).await?;
            
            // Save initial state
            let new_state = MatchState {
                id: 0, // Will be assigned by DB
                match_id,
                status: match_data.status.clone(),
                notified: true,
                notified_at: Some(Utc::now()),
                created_at: Utc::now(),
            };
            
            save_match_state(pool, &new_state).await?;
            
            // Record the notification
            record_notification(pool, match_id, "new_match", &message).await?;
        }
    }
    
    Ok(())
}

async fn send_morning_reminder(
    client: &reqwest::Client,
    pool: &SqlitePool,
    config: &Config,
) -> Result<()> {
    let message = "🌅 Good morning! Team Vitality is ready to compete today! 🎮".to_string();
    send_telegram_message(client, config, &message).await?;
    record_notification(pool, 0, "morning_reminder", &message).await?;
    
    tracing::info!("Morning reminder sent");
    Ok(())
}

async fn schedule_daily_reminder(
    client: &reqwest::Client,
    pool: &SqlitePool,
    config: &Config,
) -> Result<()> {
    let tz: Tz = config.timezone.parse()?;
    let hour = config.morning_hour;
    
    let scheduler = JobScheduler::new().await?;
    
    scheduler.add(Job::new_async("daily_reminder", move |_uuid, _l| {
        let client = client.clone();
        let pool = pool.clone();
        let config = config.clone();
        
        async move {
            // Check if it's the right day and hour
            let now: DateTime<Utc> = Utc::now();
            let local_time = now.with_timezone(&tz);
            
            if local_time.hour() == hour {
                if let Err(e) = send_morning_reminder(&client, &pool, &config).await {
                    tracing::error!("Failed to send morning reminder: {}", e);
                }
            }
        }
    })).await?;
    
    scheduler.start().await?;
    
    Ok(())
}

async fn health_check_server(port: u16) -> Result<()> {
    use warp::Filter;
    
    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({"status": "healthy"})));
    
    let routes = health.with(warp::cors().allow_any_origin());
    
    tracing::info!("Health check server starting on port {}", port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let config = Config::from_env();
    tracing::info!("Vitality Bot starting with configuration");
    
    // Initialize database
    let pool = SqlitePool::connect(&config.state_path).await?;
    initialize_database(&pool).await?;
    
    // Create HTTP client
    let client = reqwest::Client::new();
    
    // Start health check server in background
    let health_port = config.port;
    tokio::spawn(async move {
        if let Err(e) = health_check_server(health_port).await {
            tracing::error!("Health check server failed: {}", e);
        }
    });
    
    // Schedule daily reminders
    schedule_daily_reminder(&client, &pool, &config).await?;
    
    tracing::info!("Starting main polling loop...");
    
    loop {
        match check_and_notify_new_matches(&client, &pool, &config).await {
            Ok(_) => tracing::info!("Polling cycle completed successfully"),
            Err(e) => tracing::error!("Error during polling cycle: {}", e),
        }
        
        sleep(Duration::from_secs(config.poll_interval)).await;
    }
}