# Sharing with a watcher

ADR 0039 records the owner's choice that watchers see day note text. Marks,
late signs and excuse credits use the same projection and scoring as the owner.
Location and prayer method never leave the phone. A watched view labels today
using this device's civil date, since it cannot compute the owner's Fajr.

## Migration

Apply `supabase/migrations/20261029000000_sharing.sql` after the sync migration,
as database owner in the Supabase SQL editor. Do not replace the sync migration.
The sharing migration is one transaction and requests a schema cache reload.
It uses built in `gen_random_uuid` and `sha256(bytea)`, requiring PostgreSQL 13
or later, without a pgcrypto dependency.

`werdy_private.share_codes` holds hashes, owner ids, expiry and used status.
`werdy_private.share_attempts` holds a serialized failed attempt counter per
watcher. `public.werdy_shares` holds active owner and watcher pairs. Every table
has RLS enabled and no client table grants. Private schema access is revoked.
The only client access is through functions with `SECURITY DEFINER`, empty
`search_path`, qualified names, `auth.uid()` checks and authenticated grants.

RPCs are `werdy_share_create`, `werdy_share_redeem`, `werdy_share_watchers`,
`werdy_share_watched`, `werdy_share_revoke`, `werdy_share_leave` and
`werdy_watch_pull(owner_id, after_cursor, page_limit)`.
List replies expose the other person's id, email and share creation time only.
No other auth profile fields are exposed.

Codes have ten characters from an alphabet without I, O, 0 or 1. They contain
50 random bits, expire after 24 hours and can be redeemed once. At most five
unused unexpired codes exist per owner. Creation locks the owner's sync head.
Redemption locks the attempt row and code row. Ten failed attempts in a one hour
window block further attempts until that window ends. Invalid, expired, used,
self and throttled codes return the same `ok: false` result. This normal return
is necessary to commit the failed attempt counter. Exceptions would undo it.

Revoke and leave delete the active share. Each watched page locks and checks
that share, so revoke waits for an existing read and the next request after its
commit is refused. Already downloaded information cannot be recalled. Existing
sync table SELECT policies stay owner only. `werdy_sync_push` still takes its
owner from `auth.uid()`, so a watcher can only push their own records.
Watch pages contain 1 to 500 ordered operations in one JSON value, with a cursor
and `has_more`. A header split between pages waits for its rules.

## Client

Settings offers create and copy, redeem, watcher list with revoke confirmation,
and watched list with leave. Sharing requires sign in. QR is omitted.
The sharing request coordinator serializes token refresh with owner sync and
rejects completion after account generation changes. Watched data lives only
in a separate in memory `WatchView`. It has no capture or push API. It never
enters the owner's store, reminders, outbox or sync checkpoint. Read only actions
refuse mutations, mark controls are disabled, and note editing is absent.
The view refreshes every 30 seconds and on request. A refused read drops the
view, removes the person from the list and shows a plain access ended message.
Sign out drops sharing state. Network failures keep the view and offer retry.

## Owner checks

1. In SQL editor, apply the migration as database owner. Check it commits and
   reloads the RPC schema. Run `supabase/tests/sharing_checks.sql` separately.
   It uses random test users, role and JWT claim changes, and ends with rollback.
   Any failure raises an exception. If execution stops on a failure, issue
   `rollback` before another script. No SQL checks were run against a project
   during implementation.
2. With separate owner and watcher accounts, create a code. Confirm creation
   alone gives no access. Redeem it once, then confirm reuse and self redemption
   fail. Check five active codes and ten failed attempts in one hour are enforced.
3. On the emulator and web, check Arabic and English, code copying, expiry,
   invalid codes, offline failures and token refresh. Confirm owner records
   still work without signing in. No location is sent in any request.
4. Open the watched sheet and reports. Compare dates, scores, late marks, late
   notes, half and full excuse credits, note text, and counted marks. All edit
   controls must be absent or disabled. Return to own data and confirm no watched
   record was saved or reminder scheduled. Restart and confirm the cache is gone.
5. Revoke while the watcher is open, then press Refresh. The next read must fail,
   cached view disappear and list entry vanish. Repeat with Leave. Check a third
   account cannot pull, and direct table writes and forged owner pushes fail.
6. Sign out during a slow sharing request, then sign in with another account.
   Confirm late replies never restore the old code, list or viewed records.
