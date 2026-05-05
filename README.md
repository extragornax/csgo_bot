# Vitality CS:GO Bot

A bot that monitors Team Vitality's CS:GO matches and sends notifications via Telegram.

## Features

- Monitor Team Vitality matches via PandaScore API
- Send Telegram notifications for all match events
- 24-hour delayed result notifications
- Database persistence with SQLite
- Docker deployment with volume persistence
- Healthcheck endpoint
- Morning daily reminders

## Prerequisites

- Docker and Docker Compose
- PandaScore API token
- Telegram bot token and chat ID

## Setup

### 1. Environment Variables

Create a `.env` file based on the example:

```bash
cp .env.example .env
```

Edit the `.env` file with your actual values:
- `PANDASCORE_TOKEN`: Your PandaScore API token
- `VITALITY_TEAM_ID`: Team ID for Team Vitality (find from API)
- `TELEGRAM_BOT_TOKEN`: Your Telegram bot token
- `TELEGRAM_CHAT_ID`: Your Telegram chat ID

### 2. Build and Run

```bash
# Initialize data directory
./init-data.sh

# Start the bot
docker-compose up -d
```

### 3. Check Status

```bash
# View logs
docker-compose logs -f

# Check container status
docker-compose ps
```

## Data Persistence

The bot stores its state in `/app/data/state.db` which is mounted as a volume to `./data` on your host machine. This ensures that match history and previous notifications are preserved between container restarts.

## Configuration Options

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| PANDASCORE_TOKEN | - | PandaScore API token |
| VITALITY_TEAM_ID | - | Team ID for Team Vitality |
| TELEGRAM_BOT_TOKEN | - | Telegram bot token |
| TELEGRAM_CHAT_ID | - | Telegram chat ID |
| POLL_INTERVAL_SECS | 120 | Polling interval in seconds |
| MORNING_REMINDER_HOUR | 9 | Hour for morning reminders |
| TIMEZONE | Europe/Paris | Timezone for scheduling |
| PORT | 3000 | Port for healthcheck endpoint |
| STATE_PATH | /app/data/state.db | Path to SQLite database |

## Docker Compose

The `docker-compose.yml` file:
- Builds the application from the Dockerfile
- Exposes port 3000 for healthchecks
- Mounts the `./data` directory for persistent storage
- Sets up environment variables from `.env`
- Includes healthcheck configuration