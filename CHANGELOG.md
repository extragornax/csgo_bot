# Changelog

## v0.1.0 - Initial Release

### Features Implemented
- Monitor Team Vitality matches via PandaScore API
- Send Telegram notifications for all match events
- 24-hour delayed result notifications
- Database persistence with SQLite for match history
- Docker deployment with volume persistence
- Healthcheck endpoint
- Morning daily reminders
- Duplicate notification prevention (stores notification history in database)

### Deployment
- Dockerfile for building the application
- docker-compose.yml for easy deployment
- Persistent data storage in ./data directory
- Environment variable configuration
- Healthcheck integration

## v0.1.1 - Updated Docker Configuration

### Changes
- Updated Docker Compose to use port 9021 instead of 3000/3001
- Fixed .dockerignore to include Cargo.lock
- Updated README with proper port information
- Enhanced documentation for data persistence and notification history