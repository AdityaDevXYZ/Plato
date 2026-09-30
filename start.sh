#!/usr/bin/env bash
set -e

echo "=== Starting PLATO Environment ==="

# Check if postgres is running, if not, start it
if ! pg_isready -q; then
    echo "[!] PostgreSQL is not running. Starting it now..."
    # Ensure postgresql is installed
    if ! command -v pg_ctl &> /dev/null; then
        echo "Installing PostgreSQL..."
        pkg install -y postgresql
        initdb -D $PREFIX/var/lib/postgresql
    fi
    # Start the server
    pg_ctl -D $PREFIX/var/lib/postgresql -l $PREFIX/var/lib/postgresql/logfile start
    
    # Wait a few seconds for it to start
    sleep 3
else
    echo "[✓] PostgreSQL is already running."
fi

# Ensure database exists
if ! psql -lqt | cut -d \| -f 1 | grep -qw plato; then
    echo "[!] Creating 'plato' database..."
    createdb plato
fi

echo "=== Starting PLATO API ==="
# Export the environment variable for local Termux user
export PLATO__DB__URL="postgres://$(whoami)@localhost/plato"

# Run the API server
cargo run -p api
