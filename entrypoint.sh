#!/bin/sh
set -e
mkdir -p /app/data
chown app:app /app/data
exec gosu app "$@"
