-- Apply after the sharing and account deletion migrations, as database owner.
begin;

create table if not exists werdy_private.watch_requests (
    id uuid primary key default pg_catalog.gen_random_uuid(),
    sender uuid not null references auth.users(id) on delete cascade,
    receiver_email text not null check (receiver_email = pg_catalog.lower(pg_catalog.btrim(receiver_email))
        and pg_catalog.length(receiver_email) between 3 and 254),
    direction text not null check (direction in ('watch_me', 'watch_them')),
    created_at bigint not null,
    expires_at bigint not null,
    unique(sender, receiver_email, direction)
);
create index if not exists watch_requests_receiver_email on werdy_private.watch_requests(receiver_email);
create index if not exists watch_requests_sender on werdy_private.watch_requests(sender);
alter table werdy_private.watch_requests enable row level security;
revoke all on werdy_private.watch_requests from public, anon, authenticated;

create or replace function public.werdy_request_send(email text, direction text)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    now_ms bigint := public.werdy_sync_server_time();
    normalized text := pg_catalog.lower(pg_catalog.btrim(email));
    own_email text;
begin
    if direction is null or direction <> 'watch_me' or normalized is null
        or pg_catalog.length(normalized) not between 3 and 254
        or normalized !~ '^[^@[:space:]]+@[^@[:space:]]+$' then
        raise exception 'invalid watch request' using errcode = '22023';
    end if;
    -- Reuse the owner head lock, as share codes do, to serialize sends for this sender.
    insert into public.werdy_sync_heads(owner) values(caller) on conflict do nothing;
    perform 1 from public.werdy_sync_heads h where h.owner = caller for update;
    select pg_catalog.lower(u.email) into own_email from auth.users u where u.id = caller;
    if normalized = own_email then
        raise exception 'cannot request own email' using errcode = '22023';
    end if;
    if exists(select 1 from public.werdy_shares s join auth.users u on u.id = s.watcher
        where s.owner = caller and pg_catalog.lower(u.email) = normalized) then
        raise exception 'receiver already watches owner' using errcode = '22023';
    end if;
    delete from werdy_private.watch_requests r where r.expires_at <= now_ms;
    insert into werdy_private.watch_requests(sender, receiver_email, direction, created_at, expires_at)
        values(caller, normalized, direction, now_ms, now_ms + 2592000000)
        on conflict do nothing;
    if not found then raise exception 'watch request already waiting' using errcode = '22023'; end if;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

create or replace function public.werdy_request_incoming()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    now_ms bigint := public.werdy_sync_server_time();
    result jsonb;
begin
    delete from werdy_private.watch_requests r where r.expires_at <= now_ms;
    select coalesce(pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object(
        'id', r.id, 'email', sender.email, 'direction', r.direction, 'created_at', r.created_at)
        order by r.created_at, r.id), '[]'::jsonb) into result
        from werdy_private.watch_requests r join auth.users sender on sender.id = r.sender
        join auth.users receiver on receiver.id = caller
        where r.receiver_email = pg_catalog.lower(receiver.email);
    return result;
end $$;

create or replace function public.werdy_request_accept(request_id uuid)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := werdy_private.share_caller();
    now_ms bigint := public.werdy_sync_server_time();
    request werdy_private.watch_requests%rowtype;
    owner_id uuid;
begin
    delete from werdy_private.watch_requests r where r.expires_at <= now_ms;
    select r.sender into owner_id from werdy_private.watch_requests r join auth.users u on u.id = caller
        where r.id = request_id and r.receiver_email = pg_catalog.lower(u.email) and r.direction = 'watch_me';
    if owner_id is null then return pg_catalog.jsonb_build_object('ok', false); end if;
    -- Take the owner head lock first, as revoke does, so a revoke and an
    -- accept of the same share run one after the other.
    insert into public.werdy_sync_heads(owner) values(owner_id) on conflict do nothing;
    perform 1 from public.werdy_sync_heads h where h.owner = owner_id for update;
    select r.* into request from werdy_private.watch_requests r join auth.users u on u.id = caller
        where r.id = request_id and r.receiver_email = pg_catalog.lower(u.email)
        and r.expires_at > now_ms for update of r;
    if request.id is null or request.direction <> 'watch_me' then
        return pg_catalog.jsonb_build_object('ok', false);
    end if;
    insert into public.werdy_shares(owner, watcher, created_at)
        values(request.sender, caller, now_ms) on conflict do nothing;
    delete from werdy_private.watch_requests r where r.id = request.id;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

-- Revoke also ends every waiting request for the same share, in both
-- directions, so an older request cannot bring a revoked watcher back.
create or replace function public.werdy_share_revoke(watcher_id uuid)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    insert into public.werdy_sync_heads(owner) values(caller) on conflict do nothing;
    perform 1 from public.werdy_sync_heads h where h.owner = caller for update;
    delete from public.werdy_shares s where s.owner = caller and s.watcher = watcher_id;
    delete from werdy_private.watch_requests r
        where (r.sender = caller and r.direction = 'watch_me'
            and r.receiver_email = (select pg_catalog.lower(u.email) from auth.users u where u.id = watcher_id))
        or (r.sender = watcher_id and r.direction = 'watch_them'
            and r.receiver_email = (select pg_catalog.lower(u.email) from auth.users u where u.id = caller));
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

create or replace function public.werdy_request_decline(request_id uuid)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    delete from werdy_private.watch_requests r using auth.users u
        where u.id = caller and r.id = request_id and r.receiver_email = pg_catalog.lower(u.email);
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

-- Sent requests go with the sender through the cascade above. Requests waiting
-- for an email go when that account is deleted, however it is deleted. A trigger
-- keeps werdy_account_delete untouched, so other migrations may redefine it.
create or replace function werdy_private.watch_requests_forget_email()
returns trigger language plpgsql security definer set search_path = '' as $$
begin
    delete from werdy_private.watch_requests r where r.receiver_email = pg_catalog.lower(old.email);
    return old;
end $$;
revoke all on function werdy_private.watch_requests_forget_email() from public, anon, authenticated;
drop trigger if exists werdy_watch_requests_forget_email on auth.users;
create trigger werdy_watch_requests_forget_email after delete on auth.users
    for each row execute function werdy_private.watch_requests_forget_email();


revoke all on function public.werdy_request_send(text,text), public.werdy_request_incoming(),
    public.werdy_request_accept(uuid), public.werdy_request_decline(uuid)
    from public, anon, authenticated;
grant execute on function public.werdy_request_send(text,text), public.werdy_request_incoming(),
    public.werdy_request_accept(uuid), public.werdy_request_decline(uuid)
    to authenticated;
notify pgrst, 'reload schema';
commit;
