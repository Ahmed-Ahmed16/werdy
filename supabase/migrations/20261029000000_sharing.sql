-- Apply after the sync migration, as the database owner. Requires PostgreSQL 13+.
begin;
create schema if not exists werdy_private;
revoke all on schema werdy_private from public, anon, authenticated;

create table if not exists werdy_private.share_codes (
    hash bytea primary key check (pg_catalog.octet_length(hash) = 32),
    owner uuid not null references auth.users(id) on delete cascade,
    expires_at bigint not null,
    used boolean not null default false
);
create index if not exists share_codes_owner on werdy_private.share_codes(owner);
create table if not exists public.werdy_shares (
    owner uuid not null references auth.users(id) on delete cascade,
    watcher uuid not null references auth.users(id) on delete cascade,
    created_at bigint not null,
    primary key(owner, watcher),
    check (owner <> watcher)
);
create index if not exists werdy_shares_watcher on public.werdy_shares(watcher);
create table if not exists werdy_private.share_attempts (
    watcher uuid primary key references auth.users(id) on delete cascade,
    window_start bigint not null,
    failures integer not null check (failures between 0 and 10)
);
alter table werdy_private.share_codes enable row level security;
alter table public.werdy_shares enable row level security;
alter table werdy_private.share_attempts enable row level security;
-- No client table access. Only the functions below expose sharing data.
revoke all on werdy_private.share_codes, public.werdy_shares,
    werdy_private.share_attempts from public, anon, authenticated;

create or replace function werdy_private.share_caller()
returns uuid language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := auth.uid();
begin
    if caller is null then raise exception 'sign in required' using errcode = '42501'; end if;
    return caller;
end $$;

create or replace function public.werdy_share_create()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    now_ms bigint := public.werdy_sync_server_time();
    code text := '';
    alphabet constant text := 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
    entropy bytea;
    i integer;
begin
    -- Reuse the owner head lock to serialize the active code limit.
    insert into public.werdy_sync_heads(owner) values(caller) on conflict do nothing;
    perform 1 from public.werdy_sync_heads h where h.owner = caller for update;
    delete from werdy_private.share_codes c where c.owner = caller and (c.used or c.expires_at <= now_ms);
    if (select pg_catalog.count(*) from werdy_private.share_codes c where c.owner = caller) >= 5 then
        raise exception 'active share code limit reached' using errcode = '22023';
    end if;
    -- Built in UUID randomness and SHA256 need no pgcrypto extension.
    -- Ten unbiased base32 characters give 50 bits. Plain codes are never stored.
    loop
        entropy := pg_catalog.sha256(pg_catalog.convert_to(pg_catalog.gen_random_uuid()::text, 'UTF8'));
        code := '';
        for i in 0..9 loop
            code := code || pg_catalog.substr(alphabet, (pg_catalog.get_byte(entropy, i) & 31) + 1, 1);
        end loop;
        insert into werdy_private.share_codes(hash, owner, expires_at)
            values(pg_catalog.sha256(pg_catalog.convert_to(code, 'UTF8')), caller, now_ms + 86400000)
            on conflict do nothing;
        exit when found;
    end loop;
    return pg_catalog.jsonb_build_object('code', code, 'expires_at', now_ms + 86400000);
end $$;

create or replace function public.werdy_share_redeem(code text)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    now_ms bigint := public.werdy_sync_server_time();
    attempts werdy_private.share_attempts%rowtype;
    invitation werdy_private.share_codes%rowtype;
    normalized text;
begin
    insert into werdy_private.share_attempts values(caller, now_ms, 0) on conflict do nothing;
    select a.* into strict attempts from werdy_private.share_attempts a where a.watcher = caller for update;
    if now_ms >= attempts.window_start + 3600000 then
        update werdy_private.share_attempts a set window_start = now_ms, failures = 0 where a.watcher = caller;
        attempts.failures := 0;
    end if;
    if attempts.failures >= 10 then return pg_catalog.jsonb_build_object('ok', false); end if;
    if code is not null and pg_catalog.octet_length(code) <= 32 then
        normalized := pg_catalog.upper(pg_catalog.replace(pg_catalog.replace(code, ' ', ''), '-', ''));
        if normalized ~ '^[A-HJ-NP-Z2-9]{10}$' then
            select c.* into invitation from werdy_private.share_codes c
                where c.hash = pg_catalog.sha256(pg_catalog.convert_to(normalized, 'UTF8')) for update;
        end if;
    end if;
    if invitation.owner is null or invitation.owner = caller or invitation.used or invitation.expires_at <= now_ms then
        -- Return normally so failed attempts persist. SQL exceptions undo counters.
        update werdy_private.share_attempts a set failures = a.failures + 1 where a.watcher = caller;
        return pg_catalog.jsonb_build_object('ok', false);
    end if;
    insert into public.werdy_shares values(invitation.owner, caller, now_ms) on conflict do nothing;
    update werdy_private.share_codes c set used = true where c.hash = invitation.hash;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

create or replace function public.werdy_share_watchers()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller(); result jsonb;
begin
    select coalesce(pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object(
        'id', s.watcher, 'email', u.email, 'created_at', s.created_at) order by s.created_at, s.watcher), '[]'::jsonb)
        into result from public.werdy_shares s join auth.users u on u.id = s.watcher where s.owner = caller;
    return result;
end $$;

create or replace function public.werdy_share_watched()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller(); result jsonb;
begin
    select coalesce(pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object(
        'id', s.owner, 'email', u.email, 'created_at', s.created_at) order by s.created_at, s.owner), '[]'::jsonb)
        into result from public.werdy_shares s join auth.users u on u.id = s.owner where s.watcher = caller;
    return result;
end $$;

create or replace function public.werdy_share_revoke(watcher_id uuid)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    delete from public.werdy_shares s where s.owner = caller and s.watcher = watcher_id;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

create or replace function public.werdy_share_leave(owner_id uuid)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    delete from public.werdy_shares s where s.owner = owner_id and s.watcher = caller;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

create or replace function public.werdy_watch_pull(owner_id uuid, after_cursor numeric, page_limit integer)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    changes jsonb;
    next_cursor bigint;
    head bigint;
begin
    -- The share lock makes revoke wait for this read. The next call is refused
    -- after revoke commits. Sync table SELECT policies remain owner only.
    perform 1 from public.werdy_shares s where s.owner = owner_id and s.watcher = caller for share;
    if not found then raise exception 'share access denied' using errcode = '42501'; end if;
    if after_cursor is null or after_cursor < 0 or after_cursor > 9223372036854775807
        or after_cursor <> pg_catalog.trunc(after_cursor) or page_limit is null or page_limit not between 1 and 500 then
        raise exception 'invalid watch cursor or limit' using errcode = '22023';
    end if;
    select coalesce(h.last_sequence, 0) into head from public.werdy_sync_heads h where h.owner = owner_id;
    head := coalesce(head, 0);
    if after_cursor > head then raise exception 'cursor is ahead of server' using errcode = '22023'; end if;
    select coalesce(pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object('pending', c.pending,
        'stamp', pg_catalog.jsonb_build_object('received_at', c.received_at, 'sequence', c.sequence)) order by c.sequence), '[]'::jsonb),
        coalesce(pg_catalog.max(c.sequence), after_cursor::bigint)
        into changes, next_cursor from (select stream.* from public.werdy_sync_changes stream
            where stream.owner = owner_id and stream.sequence > after_cursor and stream.sequence <= head
            order by stream.sequence limit page_limit) c;
    return pg_catalog.jsonb_build_object('changes', changes, 'cursor', next_cursor, 'has_more', next_cursor < head);
end $$;

revoke all on function werdy_private.share_caller() from public, anon, authenticated;
revoke all on function public.werdy_share_create(), public.werdy_share_redeem(text),
    public.werdy_share_watchers(), public.werdy_share_watched(), public.werdy_share_revoke(uuid),
    public.werdy_share_leave(uuid), public.werdy_watch_pull(uuid,numeric,integer) from public, anon, authenticated;
grant execute on function public.werdy_share_create(), public.werdy_share_redeem(text),
    public.werdy_share_watchers(), public.werdy_share_watched(), public.werdy_share_revoke(uuid),
    public.werdy_share_leave(uuid), public.werdy_watch_pull(uuid,numeric,integer) to authenticated;
-- werdy_sync_push still derives its owner only from auth.uid(). No watcher write RPC exists.
notify pgrst, 'reload schema';
commit;
