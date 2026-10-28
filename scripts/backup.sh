#!/usr/bin/env bash
set -euo pipefail
# Disable tracing even when the caller enabled it.
set +x
umask 077

if [[ -z "${SUPABASE_DB_URL:-}" || -z "${BACKUP_AGE_RECIPIENT:-}" ]]; then
  printf 'Both backup secrets are required.\n' >&2
  exit 1
fi
if [[ $# != 1 || -e "$1" ]]; then
  printf 'Provide one new encrypted output path.\n' >&2
  exit 1
fi
output=$1
scratch=$(mktemp -d)
complete=false
cleanup() {
  rm -rf "$scratch"
  if [[ "$complete" != true ]]; then
    rm -f "$output"
  fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

# CLI errors may include credentials or server data. Keep all output local.
if ! supabase db dump --db-url "$SUPABASE_DB_URL" \
    --schema public,werdy_private --file "$scratch/schema.sql" \
    > "$scratch/cli.log" 2>&1; then
  printf 'Schema dump failed. Check the database URL and project status.\n' >&2
  exit 1
fi
if ! supabase db dump --db-url "$SUPABASE_DB_URL" --data-only --use-copy \
    --schema public,werdy_private,auth --file "$scratch/data.sql" \
    > "$scratch/cli.log" 2>&1; then
  printf 'Data dump failed. Check the database URL and project status.\n' >&2
  exit 1
fi
if [[ ! -s "$scratch/schema.sql" || ! -s "$scratch/data.sql" ]]; then
  printf 'A database dump was empty.\n' >&2
  exit 1
fi
printf 'commit=%s\nsupabase_cli=2.54.11\ncreated_utc=%s\n' \
  "${GITHUB_SHA:-unknown}" "$(date -u +%FT%TZ)" > "$scratch/manifest.txt"
if ! { tar -czf - -C "$scratch" schema.sql data.sql manifest.txt |
    age --encrypt --recipient "$BACKUP_AGE_RECIPIENT" --output "$output"; } \
    2> "$scratch/encryption.log"; then
  printf 'Archive encryption failed. Check the age recipient and disk space.\n' >&2
  exit 1
fi
complete=true
