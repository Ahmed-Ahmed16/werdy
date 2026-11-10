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

# Check the URL's shape before the CLI sees it. Each message is fixed text and
# never includes the URL or any part of it.
url_problem() {
  local url=$1 rest userinfo hostport host port
  if [[ "$url" =~ ^[[:space:]] || "$url" =~ [[:space:]]$ ]]; then
    echo 'has a space or line break at the start or end'; return
  fi
  if [[ "$url" =~ [[:space:]] ]]; then
    echo 'contains a space or line break'; return
  fi
  if [[ "$url" == \"* || "$url" == \'* ]]; then
    echo 'is wrapped in quotes; paste the URL alone'; return
  fi
  if [[ "$url" == *'[YOUR-PASSWORD]'* || "$url" == *'YOUR-PASSWORD'* ]]; then
    echo 'still has the [YOUR-PASSWORD] placeholder'; return
  fi
  if [[ "$url" != postgresql://* && "$url" != postgres://* ]]; then
    echo 'is not a Postgres connection URL (it must start with the postgresql scheme)'; return
  fi
  rest=${url#*://}
  if [[ "$rest" == *@* && "${rest%@*}" == *[/?#]* ]]; then
    echo 'has a / ? or # in the password; percent encode the password'; return
  fi
  rest=${rest%%[/?#]*}
  if [[ "$rest" != *@* ]]; then
    echo 'has no user and password before the host'; return
  fi
  userinfo=${rest%@*}
  hostport=${rest##*@}
  if [[ "$userinfo" == *@* ]]; then
    echo 'has an @ in the password; percent encode the password'; return
  fi
  local allowed="^[A-Za-z0-9._~!\$&'()*+,;=:%-]+\$"
  local encoded='^([^%]|%[0-9A-Fa-f]{2})*$'
  if [[ ! "$userinfo" =~ $allowed ]]; then
    echo 'has characters in the password that must be percent encoded'; return
  fi
  if [[ ! "$userinfo" =~ $encoded ]]; then
    echo 'has a % in the password that is not part of a percent encoding'; return
  fi
  if [[ "$userinfo" != *:* || -z "${userinfo#*:}" ]]; then
    echo 'has no password'; return
  fi
  host=${hostport%%:*}
  port=''
  [[ "$hostport" == *:* ]] && port=${hostport##*:}
  if [[ "$host" == db.*.supabase.co ]]; then
    echo 'uses the direct connection host, which GitHub runners cannot reach; use the Session pooler URL'; return
  fi
  if [[ "$port" == 6543 ]]; then
    echo 'uses port 6543 (transaction mode); use the Session pooler on port 5432'; return
  fi
  if [[ "$host" == *.pooler.supabase.com && "${userinfo%%:*}" != postgres.* ]]; then
    echo 'has a pooler host but the user is not postgres.PROJECT_REF'; return
  fi
}

# Name the cause of a CLI failure from its private log, using fixed text only.
cli_reason() {
  local log=$1
  if grep -qiE 'failed to parse|cannot parse|invalid (port|dsn|connection)|sslmode is invalid|invalid userinfo' "$log"; then
    echo 'the CLI could not parse the database URL'
  elif grep -qi 'password authentication failed' "$log"; then
    echo 'the database password was rejected'
  elif grep -qiE 'tenant or user not found' "$log"; then
    echo 'the pooler does not know this user; check postgres.PROJECT_REF and the host'
  elif grep -qiE 'no such host|server misbehaving|name resolution' "$log"; then
    echo 'the database host name was not found'
  elif grep -qiE 'network is unreachable|cannot assign requested address' "$log"; then
    echo 'the host is IPv6 only; use the Session pooler URL'
  elif grep -qiE 'config\.toml|supabase init' "$log"; then
    echo 'the CLI wanted a supabase/config.toml project file'
  elif grep -qiE 'docker' "$log"; then
    echo 'the CLI could not run Docker on the runner'
  elif grep -qiE 'timeout|timed out|connection refused|connection reset' "$log"; then
    echo 'the database did not answer; the project may be paused'
  else
    echo 'the CLI failed for an unrecognised reason'
  fi
}

problem=$(url_problem "$SUPABASE_DB_URL")
if [[ -n "$problem" ]]; then
  printf 'SUPABASE_DB_URL %s.\n' "$problem" >&2
  exit 1
fi

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
  printf 'Schema dump failed: %s.\n' "$(cli_reason "$scratch/cli.log")" >&2
  exit 1
fi
if ! supabase db dump --db-url "$SUPABASE_DB_URL" --data-only --use-copy \
    --schema public,werdy_private,auth --file "$scratch/data.sql" \
    > "$scratch/cli.log" 2>&1; then
  printf 'Data dump failed: %s.\n' "$(cli_reason "$scratch/cli.log")" >&2
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
