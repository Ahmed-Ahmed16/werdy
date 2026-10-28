# Scheduled database backups

ADR 0027 accepts Supabase Free with our own backup. The workflow
[backup.yml](../../.github/workflows/backup.yml) runs at 03:23 UTC daily and through
Actions > Database backup > Run workflow. Scheduled runs use the default branch.
GitHub may delay a scheduled run. This gives a daily recovery point when runs
succeed, not continuous recovery. Check Actions for failures and missing days.
Public repositories can have schedules disabled after 60 days without activity;
check that the workflow remains enabled. See the
[GitHub schedule rules](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule).

## What is saved

Supabase CLI 2.54.11 runs two dumps through Docker on the GitHub runner:

* `schema.sql`: `public` and `werdy_private`, including functions, constraints,
  RLS policies, ownership and grants.
* `data.sql`: `public`, `werdy_private` and `auth`, using `--data-only --use-copy`.
* `manifest.txt`: source commit, CLI version and archive creation time in UTC.

Current app tables are `public.werdy_sync_heads`, `public.werdy_sync_changes`,
`public.werdy_shares`, `werdy_private.share_codes` and
`werdy_private.share_attempts`. Whole app schemas are selected so a later table
in either schema is included too. Schema and data dumps use separate snapshots.
Do not apply migrations during a backup. All selected data shares one data dump
snapshot, so ordinary concurrent app writes do not split related rows.

Auth data includes `auth.users` and `auth.identities`, preserving UUID foreign
keys, emails and Google identity links. We keep the rest of auth data too,
including sessions and audit records, rather than maintain a partial list of
auth dependencies. This broad scope is sensitive. The CLI excludes
`auth.schema_migrations`; the new project's managed auth schema supplies that
history. We do not dump the auth schema or managed roles. Current migrations
use Supabase's existing roles and do not customize auth objects. Custom roles
or auth schema changes would require revisiting this procedure.

Storage files, other schemas, project settings, Google provider secrets, JWT
signing keys and unsynced device records are not included. Configure sign in
and app endpoints separately after recovery. No Supabase access token or
service role key is needed by the workflow.

The script keeps SQL and CLI errors in a private temporary directory and removes
it on success or failure. CLI output never reaches the job log. A compressed
archive is piped to age on the runner. Only `werdy-backup.tar.gz.age` is uploaded,
with 30 days retention. This repo is public, so treat artifact downloads as
public. Encryption protects the user data and emails. There is no plaintext
artifact or cache. Failure leaves
no upload. The job fails if either secret is absent. The offline private key
never enters GitHub. Keep an encrypted copy outside GitHub too if recovery must
survive repository deletion or artifact expiry.

## Owner setup: you should type this

Use a trusted machine. Commands below use Bash on Ubuntu 24.04, including WSL.
For Windows without WSL, age 1.2.1 has a Windows binary in its
[release](https://github.com/FiloSottile/age/releases/tag/v1.2.1).
Install age for key creation. Recovery also needs `unzip`, `tar` and `psql`:

```bash
sudo apt-get update
sudo apt-get install --yes age unzip postgresql-client
```

1. On the machine that will hold the offline key, connect an encrypted removable
   drive and choose a new directory on it. Set the path below to that directory.
   Do not use the repo, a cloud folder, or a shared machine. Run:

   ```bash
   set +x
   umask 077
   read -r -p 'Offline key directory: ' key_dir
   mkdir -p "$key_dir"
   age-keygen -o "$key_dir/werdy-backup-key.txt"
   age-keygen -y "$key_dir/werdy-backup-key.txt"
   ```

   The last command prints only the public recipient, starting with `age1`.
   Copy that public line. Keep the private file offline, with a second secure
   offline copy. Losing it makes every backup encrypted to it unreadable.
   Keep old keys until all backups encrypted to them have expired.
2. In the source Supabase project, open Database > Settings and reset the
   database password. Save the new password in your password manager. A reset
   also affects other tools using the old database password.
3. Click Connect at the top of the project. Select URI and Session pooler.
   Copy its exact host and username. Session mode uses port `5432` and username
   `postgres.PROJECT_REF`. It works on the GitHub runner's IPv4 network. Do not
   use transaction mode on port `6543`, or the API project URL.
4. Replace `[YOUR-PASSWORD]` with the reset database password, percent encoding
   reserved characters in the password only. If needed, Ubuntu's Python 3 can
   encode it without entering the password in shell history:

   ```bash
   python3 -c 'import getpass, urllib.parse; print(urllib.parse.quote(getpass.getpass("Database password: "), safe=""))'
   ```

   This prints an encoded password. Treat it as a secret and clear the terminal
   and clipboard afterwards. The URL shape is:

   ```text
   postgresql://postgres.PROJECT_REF:ENCODED_PASSWORD@POOLER_HOST:5432/postgres?sslmode=require
   ```

   Use values copied from Connect, not the placeholders. Add `?sslmode=require`
   if there is no query already. If there is one, append `&sslmode=require`.
5. In GitHub, open Ahmed-Ahmed16/werdy > Settings > Secrets and variables >
   Actions > New repository secret. Add `SUPABASE_DB_URL` with the complete URL.
   Add `BACKUP_AGE_RECIPIENT` with only the public `age1` line. Never add the
   private key. No environment secret or app client setting is used here.
6. After the workflow reaches `main`, open Actions > Database backup > Run
   workflow, choose `main`, and run it. Confirm a successful encrypted artifact
   exists, then do the restore test below. Check later daily runs too. A paused
   project or wrong password makes a dump fail. Resume the project or correct
   the secret, then run the workflow again. Errors are intentionally generic;
   never add debug tracing or print SQL or URLs to diagnose them.

## Download and decrypt

In Actions, open a successful Database backup run. Under Artifacts, download
`werdy-backup-RUN_ID-RUN_ATTEMPT`. GitHub downloads a ZIP containing the age file.
Work outside the repo in a private directory on an encrypted disk. Use absolute
ZIP and offline key directory paths when prompted:

```bash
set -euo pipefail
set +x
umask 077
read -r -p 'Downloaded artifact ZIP path: ' artifact_zip
read -r -p 'New private recovery directory: ' recovery_dir
mkdir "$recovery_dir"
unzip "$artifact_zip" -d "$recovery_dir"
cd "$recovery_dir"
read -r -p 'Offline key directory: ' key_dir
age --decrypt --identity "$key_dir/werdy-backup-key.txt" \
  --output backup.tar.gz werdy-backup.tar.gz.age
tar -xzf backup.tar.gz
cat manifest.txt
```

Stop if decryption or extraction fails. Use the matching old private key after
a key rotation. Never upload decrypted files or paste their contents into an
issue. Disconnect the key drive when decryption is finished.

## Restore into an empty project

These commands write to the chosen database. Use a separate empty Supabase
project for the test, never the source project. Do not connect app clients or
sign in until restore finishes. Do not apply the repo migrations first: the
schema dump already contains them. CLI migration history in `supabase_migrations`
is not saved. Do not run `supabase db push` until its history is reconciled with
the restored schema. Use the source commit in `manifest.txt` for
the app and migration context.

1. Create an empty project with a compatible Postgres major version and managed
   auth layout. The pinned CLI uses its bundled Postgres dump tools. A future
   source version beyond those tools may need a CLI update. An older backup's
   auth columns must match the target for COPY restore. Record source and target
   versions during the owner test. Compatibility has not been tested here.
2. Reset the target database password. Copy its Session pooler URL through
   Connect and fill in the encoded password as above. Verify its project ref
   differs from the source. From the decrypted recovery directory, run the
   following. The hidden prompt avoids shell history; do not enable tracing.
   `psql -X` ignores local startup files. Errors stop and roll back the whole
   restore transaction. Keep logs local because SQL errors may expose row data.

   ```bash
   read -r -s -p 'EMPTY TARGET session pooler URL: ' target_db_url
   printf '\n'
   psql -X --dbname "$target_db_url" --single-transaction \
     --set ON_ERROR_STOP=on \
     --file schema.sql \
     --command 'SET session_replication_role = replica;' \
     --file data.sql \
     --command 'SET session_replication_role = origin;' \
     --command "NOTIFY pgrst, 'reload schema';" \
     > restore.log 2>&1
   ```

   Use only an empty target. The replica setting avoids auth triggers and
   foreign key ordering failures while loading the preserved rows. It does not
   remove RLS policies or grants. Any error requires investigation in the
   private `restore.log`. Do not skip errors or strip constraints to proceed.
   If auth schema versions differ, use a compatible target or revise and test
   a column mapping before trying again. Do not retry against a partial target
   if a different restore tool was used without a transaction.
3. Verify counts locally and check references after triggers are enabled again:

   ```bash
   psql -X --dbname "$target_db_url" --set ON_ERROR_STOP=on <<'SQL'
   select 'auth.users' as relation, count(*) from auth.users
   union all select 'auth.identities', count(*) from auth.identities
   union all select 'public.werdy_sync_heads', count(*) from public.werdy_sync_heads
   union all select 'public.werdy_sync_changes', count(*) from public.werdy_sync_changes
   union all select 'public.werdy_shares', count(*) from public.werdy_shares
   union all select 'werdy_private.share_codes', count(*) from werdy_private.share_codes
   union all select 'werdy_private.share_attempts', count(*) from werdy_private.share_attempts;
   select count(*) as orphaned_owners from (
     select owner as id from public.werdy_sync_heads
     union all select owner from public.werdy_sync_changes
     union all select owner from public.werdy_shares
     union all select watcher from public.werdy_shares
     union all select owner from werdy_private.share_codes
     union all select watcher from werdy_private.share_attempts
     union all select user_id from auth.identities
   ) refs left join auth.users users on users.id = refs.id where users.id is null;
   select schemaname, tablename, rowsecurity from pg_tables
   where schemaname in ('public', 'werdy_private');
   SQL
   psql -X --dbname "$target_db_url" --set ON_ERROR_STOP=on \
     --file /path/to/werdy/supabase/tests/sync.sql
   psql -X --dbname "$target_db_url" --set ON_ERROR_STOP=on \
     --file /path/to/werdy/supabase/tests/sharing_checks.sql
   unset target_db_url
   ```

   Replace `/path/to/werdy` with the checkout of the manifest commit. Both SQL
   checks roll back their fixtures. Expect zero orphaned owners and RLS true
   for all five app tables. Compare counts with a source count recorded while
   app writes were paused for the test, not with a later changing live count.
4. Recreate Google provider settings, redirect URLs and other project settings
   manually. Use the new project URL and publishable key in a test app build.
   Use a fresh sign in rather than relying on restored sessions. Confirm the
   original user UUID, deeds, marks and notes return through sync, and sharing
   still permits only the intended watcher. Confirm unrelated users are denied.
   Restore does not recover Google credentials or project JWT signing keys.
5. Save the private test result, then remove plaintext SQL, archive and logs
   from the recovery disk when no longer needed. Do not rely on file deletion
   to erase an unencrypted disk. Point `SUPABASE_DB_URL` at the replacement only
   when it becomes the chosen backend, then test its next backup.

- [ ] Owner restore test: download one scheduled encrypted artifact, decrypt
  with the offline key, restore into an empty project, compare counts and UUIDs,
  pass both SQL checks and fresh Google sign in, sync and watcher checks.
  Date: `___`. Run id: `___`. Source and target Postgres/auth versions: `___`.
  Result and private evidence location: `___`.

## Local checks and sources

With age installed, run `bash scripts/tests/backup.sh` from the repo root.
It replaces the database CLI with a fixture, uses real age and tar, and checks
missing secrets, both dump failures, invalid recipients, plaintext cleanup,
log privacy and decryption of both dump files. It does not prove a live restore.
Validate the workflow with `actionlint .github/workflows/backup.yml`.

Command behavior was checked against the pinned
[CLI flags](https://github.com/supabase/cli/blob/v2.54.11/cmd/db.go),
[data dump script](https://github.com/supabase/cli/blob/v2.54.11/pkg/migration/scripts/dump_data.sh)
and [schema dump script](https://github.com/supabase/cli/blob/v2.54.11/pkg/migration/scripts/dump_schema.sh).
Connection and restore steps follow Supabase's
[connection guide source](https://github.com/supabase/supabase/blob/master/apps/docs/content/guides/database/connecting-to-postgres.mdx)
and [restore guide source](https://github.com/supabase/supabase/blob/master/apps/docs/content/guides/platform/migrating-within-supabase/backup-restore.mdx).
No Supabase project or Google endpoint was contacted during implementation.
