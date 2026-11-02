#!/usr/bin/env bash
set -e

echo "🛰️  ================================================================"
echo "🛰️  PLATO: Mission Control for Commercial Satellite Constellations"
echo "🛰️  ================================================================"

if [ "$1" == "--docker" ]; then
    echo "[+] Launching multi-container production stack via Docker Compose..."
    docker compose up -d --build
    echo "[✓] PLATO is live!"
    echo "    - Mission Control Frontend: http://localhost:3000"
    echo "    - REST & WebSocket API:     http://localhost:8080"
    echo "    - Operator Login:           admin@plato.aero / PlatoAdmin2026!"
    exit 0
fi

echo "[+] Initializing Local PLATO Environment..."

# Check if PostgreSQL is running, start if needed
if ! pg_isready -q 2>/dev/null; then
    echo "[!] PostgreSQL is not running. Starting local database server..."
    if command -v pg_ctl &> /dev/null; then
        PGDATA_DIR="${PGDATA:-$PREFIX/var/lib/postgresql}"
        if [ ! -d "$PGDATA_DIR" ]; then
            initdb -D "$PGDATA_DIR"
        fi
        pg_ctl -D "$PGDATA_DIR" -l "$PGDATA_DIR/logfile" start
        sleep 2
    else
        echo "[x] Error: PostgreSQL not found. Please install PostgreSQL or run with --docker."
        exit 1
    fi
else
    echo "[✓] PostgreSQL is running and ready."
fi

# Ensure 'plato' database exists
if ! psql -lqt 2>/dev/null | cut -d \| -f 1 | grep -qw plato; then
    echo "[!] Creating 'plato' database..."
    createdb plato
fi

echo "[✓] Database ready. Configuring environment variables..."
export PLATO__DB__URL="${DATABASE_URL:-postgres://$(whoami)@localhost/plato}"
export DATABASE_URL="$PLATO__DB__URL"
export PLATO__SERVER__HOST="${PLATO__SERVER__HOST:-127.0.0.1}"
export PLATO__SERVER__PORT="${PLATO__SERVER__PORT:-8080}"
export RUST_LOG="${RUST_LOG:-info,api=debug}"

echo "=================================================================="
echo "🚀 Launching PLATO API Server & Genesis Bootstrapper..."
echo "   Default Admin: admin@plato.aero / PlatoAdmin2026!"
echo "=================================================================="

exec cargo run -p api
