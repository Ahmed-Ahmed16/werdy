#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
test -f "$root/scripts/backup.sh"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/bin" "$scratch/tmp"
export TMPDIR="$scratch/tmp"
export PATH="$scratch/bin:$PATH"
good_url='postgresql://postgres.testref:secret@aws-0-test.pooler.supabase.com:5432/postgres?sslmode=require'
export SUPABASE_DB_URL=$good_url
age-keygen -o "$scratch/key" > /dev/null 2>&1
BACKUP_AGE_RECIPIENT=$(age-keygen -y "$scratch/key")
export BACKUP_AGE_RECIPIENT

# Only the database boundary is replaced. Encryption and archive tools are real.
cat > "$scratch/bin/supabase" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
kind=schema
file=''
schemas=''
use_copy=false
while (($#)); do
  case "$1" in
    --data-only) kind=data; shift ;;
    --file) file=$2; shift 2 ;;
    --schema) schemas=$2; shift 2 ;;
    --use-copy) use_copy=true; shift ;;
    *) shift ;;
  esac
done
if [[ "$kind" == schema ]]; then
  test "$schemas" = public,werdy_private
else
  test "$schemas" = public,werdy_private,auth
  test "$use_copy" = true
fi
if [[ "${FAIL_DUMP:-}" == "$kind" ]]; then
  printf '%s\n' "$SUPABASE_DB_URL private@example.invalid ${FAIL_TEXT:-}" >&2
  exit 1
fi
printf '%s\n' "$kind private@example.invalid" > "$file"
STUB
chmod +x "$scratch/bin/supabase"

for scenario in missing_url missing_recipient schema data invalid_recipient; do
  (
    case "$scenario" in
      missing_url) unset SUPABASE_DB_URL ;;
      missing_recipient) unset BACKUP_AGE_RECIPIENT ;;
      schema|data) export FAIL_DUMP="$scenario" ;;
      invalid_recipient) export BACKUP_AGE_RECIPIENT='invalid' ;;
    esac
    if bash "$root/scripts/backup.sh" "$scratch/backup.age" > "$scratch/log" 2>&1; then
      printf 'FAIL: %s unexpectedly succeeded\n' "$scenario" >&2
      exit 1
    fi
    test ! -e "$scratch/backup.age"
    test -z "$(ls -A "$TMPDIR")"
    if grep -Eq 'postgresql://|private@example.invalid' "$scratch/log"; then
      printf 'FAIL: %s leaked private output\n' "$scenario" >&2
      exit 1
    fi
  )
  printf 'PASS: %s fails without output or residue\n' "$scenario"
done

leaks() {
  grep -Eq 'postgresql://|private@example.invalid|secret|testref|hunter' "$1"
}

# A malformed URL stops before the CLI runs, with a fixed reason and no leak.
url_case() {
  local name=$1 url=$2 expected=$3
  (
    export SUPABASE_DB_URL=$url FAIL_DUMP=schema
    if bash "$root/scripts/backup.sh" "$scratch/backup.age" > "$scratch/log" 2>&1; then
      printf 'FAIL: url %s unexpectedly succeeded\n' "$name" >&2
      exit 1
    fi
    test ! -e "$scratch/backup.age"
    test -z "$(ls -A "$TMPDIR")"
    if ! grep -qF "$expected" "$scratch/log" || leaks "$scratch/log"; then
      printf 'FAIL: url %s gave the wrong reason or leaked\n' "$name" >&2
      exit 1
    fi
  )
  printf 'PASS: url %s is named without leaking\n' "$name"
}
url_case trailing_newline "$good_url"$'\n' 'space or line break at the start or end'
url_case quoted "\"$good_url\"" 'wrapped in quotes'
url_case placeholder 'postgresql://postgres.testref:[YOUR-PASSWORD]@aws-0-test.pooler.supabase.com:5432/postgres' 'placeholder'
url_case api_url 'https://testref.supabase.co' 'is not a Postgres connection URL'
url_case raw_at 'postgresql://postgres.testref:hunter@2@aws-0-test.pooler.supabase.com:5432/postgres' 'has an @ in the password'
url_case raw_hash 'postgresql://postgres.testref:hunter#2@aws-0-test.pooler.supabase.com:5432/postgres' 'has a / ? or # in the password'
url_case raw_space 'postgresql://postgres.testref:hunter 2@aws-0-test.pooler.supabase.com:5432/postgres' 'contains a space'
url_case bad_percent 'postgresql://postgres.testref:hunter%zz@aws-0-test.pooler.supabase.com:5432/postgres' 'not part of a percent encoding'
url_case direct_host 'postgresql://postgres:hunter@db.testref.supabase.co:5432/postgres' 'direct connection host'
url_case transaction_port 'postgresql://postgres.testref:hunter@aws-0-test.pooler.supabase.com:6543/postgres' 'port 6543'
url_case pooler_user 'postgresql://postgres:hunter@aws-0-test.pooler.supabase.com:5432/postgres' 'not postgres.PROJECT_REF'

# A CLI failure is named from its private log with fixed text, never echoed.
reason_case() {
  local kind=$1 text=$2 expected=$3
  (
    export FAIL_DUMP=$kind FAIL_TEXT=$text
    if bash "$root/scripts/backup.sh" "$scratch/backup.age" > "$scratch/log" 2>&1; then
      printf 'FAIL: reason %s unexpectedly succeeded\n' "$expected" >&2
      exit 1
    fi
    test ! -e "$scratch/backup.age"
    test -z "$(ls -A "$TMPDIR")"
    if ! grep -qF "$expected" "$scratch/log" || leaks "$scratch/log" \
        || grep -qF "$text" "$scratch/log"; then
      printf 'FAIL: reason %s was wrong or leaked\n' "$expected" >&2
      exit 1
    fi
  )
  printf 'PASS: %s dump failure names "%s"\n' "$kind" "$expected"
}
reason_case schema 'failed to parse connection string: cannot parse' 'could not parse the database URL'
reason_case schema 'FATAL: password authentication failed for user' 'password was rejected'
reason_case schema 'FATAL: Tenant or user not found' 'pooler does not know this user'
reason_case schema 'dial tcp: lookup x: no such host' 'host name was not found'
reason_case schema 'dial tcp [2a05::1]:5432: connect: network is unreachable' 'IPv6 only'
reason_case schema 'open supabase/config.toml: no such file. Have you set up the project with supabase init?' 'supabase/config.toml'
reason_case schema 'Cannot connect to the Docker daemon' 'could not run Docker'
reason_case data 'dial tcp 1.2.3.4:5432: i/o timeout' 'may be paused'
reason_case data 'something new' 'unrecognised reason'

bash "$root/scripts/backup.sh" "$scratch/backup.age" > "$scratch/log" 2>&1
test -z "$(ls -A "$TMPDIR")"
age --decrypt -i "$scratch/key" -o "$scratch/backup.tar.gz" "$scratch/backup.age"
mkdir "$scratch/restored"
tar -xzf "$scratch/backup.tar.gz" -C "$scratch/restored"
grep -qx 'schema private@example.invalid' "$scratch/restored/schema.sql"
grep -qx 'data private@example.invalid' "$scratch/restored/data.sql"
if leaks "$scratch/log"; then
  printf 'FAIL: success leaked private output\n' >&2
  exit 1
fi
printf 'PASS: encrypted archive decrypts to both dumps without log leaks\n'
