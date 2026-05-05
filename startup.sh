#!/bin/bash
# Startup script for vitality bot

# Ensure data directory exists and has proper permissions
mkdir -p /app/data
chmod 755 /app/data

# If state.db doesn't exist, create it with proper permissions
if [ ! -f "/app/data/state.db" ]; then
    touch /app/data/state.db
    chmod 666 /app/data/state.db
fi

# Start the main application
exec /usr/local/bin/vitality_bot