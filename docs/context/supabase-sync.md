# Supabase sync

The optional `werdy-sync` feature `supabase` provides sync and auth transports.
Stage C wires sign in and background sync into the app. The adapter takes a project origin, a public `sb_publishable_`
key and the owner's access token. Each RPC sends `apikey` and
`Authorization: Bearer <access token>`. Replace a refreshed token with
`SupabaseBackend::set_access_token`. `SupabaseAuth` exchanges Google credentials
and refreshes sessions. The app refreshes two minutes early, or once after a 401.
Native requests need the caller's Tokio runtime. The dev dependency on Tokio
runs the ignored native HTTP test. Browser requests use fetch.

## Database

`supabase/migrations/20261028000000_sync.sql` creates two tables:

* `werdy_sync_heads`: one head per `auth.users.id`, with the last sequence and
  last accepted server time in Unix milliseconds.
* `werdy_sync_changes`: the ordered stream and permanent retry receipts. The
  unique key `(owner, install, operation)` rejects a retry with a changed payload.
  Account deletion removes both tables' rows for that owner through foreign keys.

The push RPC locks the owner's head row. Only a new operation advances its
sequence. A rollback restores the head and all inserts, so committed sequences
have no gaps. Accepted time is the greater of the current server time and the
previous accepted time. Equal times use sequence order. Retries keep their
original stamps and return in request order.

The RPCs are `werdy_sync_push(batch jsonb)`,
`werdy_sync_pull(after_cursor numeric)` and `werdy_sync_server_time()`.
Push and pull return one JSON object with `changes` and `cursor`. Pull reads
through a captured head and aggregates the complete stream into that one value.
PostgREST's row limit cannot trim individual changes from it. It is not paged;
very long histories may need a later protocol change to bound response memory.

Both tables enable RLS. Authenticated owners can select only their own rows.
Clients have no insert, update, delete or truncate grants. There are no write
policies. The three RPCs use `SECURITY DEFINER` with an empty `search_path`,
qualified object names and an explicit `auth.uid()` filter. Null identity fails.
Execute is granted only to `authenticated`, after revoking it from `PUBLIC` and
`anon`. The private validation helpers have no client grants. Apply as the
database owner so the definer has the required table access. See the official
[function security guide](https://supabase.com/docs/guides/database/functions).

Only known wire fields and tags are accepted, including nested fields.
Location and prayer method stay on the device under ADR 0010. Neither is an
accepted server setting. Text notes are still the owner's own text; the server
does not inspect their meaning.

## Limits

* At most 1024 operations and 1 MiB per push, measured as PostgreSQL `jsonb::text`.
* At most 16 KiB per operation in that same representation.
* Install ids: 1 to 128 ASCII letters, digits or underscores.
* Keys and target units: 1 to 128 UTF8 bytes. Names: at most 1024 bytes.
* Notes: at most 8192 UTF8 bytes. Language: 1 to 16 bytes.
* At most 4096 archive periods or dated calendar offsets. Weekdays are unique
  and at most seven. Rules, dates, fractions, targets and enum values are checked.
* Operation ids and deed counters cover Rust `u64`. Fractions cover `u128`.
  Server sequences use positive PostgreSQL `bigint`, through 9223372036854775807.
* Requests time out after 30 seconds. Transport and JSON errors omit credentials
  and response bodies. Native HTTP redirects are disabled. Browser fetch uses
  its own redirect behavior; pass the trusted project origin directly.

Capture refuses new notes, names and keys outside their byte limits. Legacy
oversized outbox rows stay local, with logged reasons, until corrected. Related
deed rows stay local together so they cannot create incomplete server deeds.
Other server limit failures still return errors and keep the outbox.

Push targets 32 operations per request, keeping each deed header with all its
pending rules. A larger deed group travels in one request and remains subject
to the server operation and byte limits above. Each request is atomic. A later
failed batch keeps the whole outbox; retries deduplicate the earlier batches.

## Apply

In the Supabase dashboard, open SQL Editor and run the whole migration as the
database owner. It creates objects, grants access and requests a PostgREST schema
cache reload in one transaction. It can be applied again to the same layout.

Or use the [Supabase CLI](https://supabase.com/docs/reference/cli/supabase-db-push).
From the repo root, initialize CLI config with `supabase init` if needed, then
run `supabase link --project-ref <project ref>` and `supabase db push` against the
chosen project. These commands apply SQL to that project. The adapter never
needs a database password or a service role key.

## Verify

Offline Rust checks use `cargo test -p werdy-sync --features supabase`.
The request and response tests use canned JSON, including the shared SQL wire
fixture. `supabase/tests/sync.sql` checks RLS, grants, receipts, rollback, clock
order and input rejection. Run it as the database owner in a test database with
the migration applied, for example `psql "$env:WERDY_TEST_DATABASE_URL" -v ON_ERROR_STOP=1 -f supabase/tests/sync.sql`.
It creates test users and rolls back all test changes. It does not exercise
concurrent database sessions or PostgREST itself.

The ignored test appends one harmless set removal under a unique test key.
Use a dedicated test account on a project with the migration applied. Supply
`WERDY_SUPABASE_URL`, `WERDY_SUPABASE_KEY` and `WERDY_SUPABASE_TOKEN` through the
environment. The key must be a public publishable key. Do not commit a token.

```powershell
cargo test -p werdy-sync --features supabase --test supabase_live -- --ignored --exact supabase_round_trip
```

Missing variables fail before any request. HTTP failures report a status code;
check the token and migration, then retry. An ambiguous push failure is safe to
retry because its operation identity and original server receipt are kept.
