#!/usr/bin/env bash
# ==============================================================================
# NebuDCS Automated Production Backup Script
# Triggers cryptographic backup bundle export and rotates older archives.
# Typically scheduled via cron: 0 2 * * * /usr/local/bin/nebudcs-backup.sh
# ==============================================================================

set -euo pipefail

# Configuration
CONFIG_FILE="${CONFIG_FILE:-/etc/nebudcs/nebudcs.env}"
BACKUP_DIR="${BACKUP_DIR:-/var/backups/nebudcs}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"

# Load environment configuration if available
if [[ -f "$CONFIG_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$CONFIG_FILE"
fi

HOST_URL="${HOST_URL:-http://127.0.0.1:${PORT:-8080}}"

TOKEN="${ADMIN_TOKEN:-${AUTH_TOKEN:-}}"
if [[ -z "$TOKEN" ]]; then
    echo "ERROR: Neither ADMIN_TOKEN nor AUTH_TOKEN is set. Cannot authenticate backup request." >&2
    exit 1
fi

mkdir -p "$BACKUP_DIR"
TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
TARGET_FILE="${BACKUP_DIR}/nebudcs_backup_${TIMESTAMP}.json"

echo "[$(date -u)] Requesting backup bundle from ${HOST_URL}..."
HTTP_STATUS=$(curl -s -w "%{http_code}" -o "$TARGET_FILE" \
    -X POST "${HOST_URL}/api/v1/admin/backup" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json")

if [[ "$HTTP_STATUS" != "200" ]]; then
    echo "ERROR: Backup request failed with HTTP status ${HTTP_STATUS}" >&2
    rm -f "$TARGET_FILE"
    exit 1
fi

FILE_SIZE=$(wc -c < "$TARGET_FILE")
echo "[$(date -u)] Backup successful: ${TARGET_FILE} (${FILE_SIZE} bytes)"

# Clean up older backups past retention window
echo "[$(date -u)] Rotating backups older than ${RETENTION_DAYS} days..."
find "$BACKUP_DIR" -name "nebudcs_backup_*.json" -type f -mtime +"$RETENTION_DAYS" -exec rm -f {} +

echo "[$(date -u)] Backup and rotation complete."
