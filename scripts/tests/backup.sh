#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
test -f "$root/scripts/backup.sh"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/bin" "$scratch/tmp"
export TMPDIR="$scratch/tmp"
export PATH="$scratch/bin:$PATH"
export SUPABASE_DB_URL='postgresql://test:secret@example.invalid/postgres'
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
  printf '%s\n' "$SUPABASE_DB_URL private@example.invalid" >&2
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

bash "$root/scripts/backup.sh" "$scratch/backup.age" > "$scratch/log" 2>&1
test -z "$(ls -A "$TMPDIR")"
age --decrypt -i "$scratch/key" -o "$scratch/backup.tar.gz" "$scratch/backup.age"
mkdir "$scratch/restored"
tar -xzf "$scratch/backup.tar.gz" -C "$scratch/restored"
grep -qx 'schema private@example.invalid' "$scratch/restored/schema.sql"
grep -qx 'data private@example.invalid' "$scratch/restored/data.sql"
if grep -Eq 'postgresql://|private@example.invalid' "$scratch/log"; then
  printf 'FAIL: success leaked private output\n' >&2
  exit 1
fi
printf 'PASS: encrypted archive decrypts to both dumps without log leaks\n'
