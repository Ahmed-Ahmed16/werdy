-- Apply after the account deletion migration, as the database owner.
begin;
create schema if not exists werdy_private;
revoke all on schema werdy_private from public, anon, authenticated;

create table if not exists public.werdy_push_subscriptions (
    id uuid primary key default pg_catalog.gen_random_uuid(),
    owner uuid not null references auth.users(id) on delete cascade,
    endpoint text not null unique check (pg_catalog.octet_length(endpoint) <= 2048
        and endpoint ~ '^https://[^[:space:]]+$'),
    p256dh text not null,
    auth text not null check (auth ~ '^[A-Za-z0-9_-]{1,128}$'),
    created_at timestamptz not null default now()
);
-- Replace the old constraint so a repeated apply also fixes its regex.
alter table public.werdy_push_subscriptions
    drop constraint if exists werdy_push_subscriptions_p256dh_check;
alter table public.werdy_push_subscriptions
    add constraint werdy_push_subscriptions_p256dh_check
    check (p256dh ~ '^[A-Za-z0-9_-]+$' and pg_catalog.char_length(p256dh) between 1 and 256);
create index if not exists werdy_push_subscriptions_owner on public.werdy_push_subscriptions(owner);

create table if not exists public.werdy_push_reminders (
    subscription uuid not null references public.werdy_push_subscriptions(id) on delete cascade,
    reminder text not null check (pg_catalog.char_length(reminder) between 1 and 128),
    send_at timestamptz not null,
    title text not null check (pg_catalog.char_length(title) <= 120),
    body text not null check (pg_catalog.char_length(body) <= 240),
    topic text not null check (topic ~ '^[A-Za-z0-9_-]{1,32}$'),
    primary key (subscription, reminder)
);
create index if not exists werdy_push_reminders_send_at on public.werdy_push_reminders(send_at);

alter table public.werdy_push_subscriptions enable row level security;
alter table public.werdy_push_reminders enable row level security;
revoke all on table public.werdy_push_subscriptions, public.werdy_push_reminders
    from public, anon, authenticated;
grant select on table public.werdy_push_subscriptions, public.werdy_push_reminders to authenticated;
drop policy if exists werdy_push_subscriptions_select on public.werdy_push_subscriptions;
create policy werdy_push_subscriptions_select on public.werdy_push_subscriptions
    for select to authenticated using (owner = (select auth.uid()));
drop policy if exists werdy_push_reminders_select on public.werdy_push_reminders;
create policy werdy_push_reminders_select on public.werdy_push_reminders
    for select to authenticated using (exists (
        select 1 from public.werdy_push_subscriptions s
        where s.id = subscription and s.owner = (select auth.uid())
    ));

create or replace function werdy_private.push_valid_subscription(v jsonb)
returns boolean language plpgsql immutable set search_path = '' as $$
begin
    if not werdy_private.sync_object(v, array['endpoint','p256dh','auth']) then return false; end if;
    return pg_catalog.jsonb_typeof(v->'endpoint') = 'string'
        and pg_catalog.jsonb_typeof(v->'p256dh') = 'string'
        and pg_catalog.jsonb_typeof(v->'auth') = 'string'
        and pg_catalog.octet_length(v->>'endpoint') <= 2048
        and v->>'endpoint' ~ '^https://[^[:space:]]+$'
        and v->>'p256dh' ~ '^[A-Za-z0-9_-]+$'
        and pg_catalog.char_length(v->>'p256dh') between 1 and 256
        and v->>'auth' ~ '^[A-Za-z0-9_-]{1,128}$';
end $$;

create or replace function werdy_private.push_valid_reminder(v jsonb, earliest_ms numeric, latest_ms numeric)
returns boolean language plpgsql immutable set search_path = '' as $$
declare send_ms numeric;
begin
    if not werdy_private.sync_object(v, array['id','send_at_ms','title','body','topic']) then return false; end if;
    if pg_catalog.jsonb_typeof(v->'id') <> 'string'
        or pg_catalog.jsonb_typeof(v->'send_at_ms') <> 'number'
        or pg_catalog.jsonb_typeof(v->'title') <> 'string'
        or pg_catalog.jsonb_typeof(v->'body') <> 'string'
        or pg_catalog.jsonb_typeof(v->'topic') <> 'string' then return false; end if;
    send_ms := (v->>'send_at_ms')::numeric;
    return pg_catalog.char_length(v->>'id') between 1 and 128
        and send_ms = pg_catalog.trunc(send_ms)
        and send_ms between earliest_ms and latest_ms
        and pg_catalog.char_length(v->>'title') <= 120
        and pg_catalog.char_length(v->>'body') <= 240
        and v->>'topic' ~ '^[A-Za-z0-9_-]{1,32}$';
end $$;

create or replace function public.werdy_push_replace(subscription jsonb, reminders jsonb)
returns void language plpgsql volatile security definer set search_path = '' as $$
declare
    caller uuid := auth.uid();
    now_ms numeric := pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000);
    subscription_id uuid;
    item jsonb;
begin
    if caller is null then raise exception 'sign in required' using errcode = '42501'; end if;
    if not werdy_private.push_valid_subscription(subscription) then
        raise exception 'invalid push subscription' using errcode = '22023';
    end if;
    if reminders is null or pg_catalog.jsonb_typeof(reminders) <> 'array' then
        raise exception 'reminders must be an array' using errcode = '22023';
    end if;
    if pg_catalog.jsonb_array_length(reminders) > 50 then
        raise exception 'reminder limit exceeded' using errcode = '22023';
    end if;
    for item in select pg_catalog.jsonb_array_elements(reminders) loop
        if not werdy_private.push_valid_reminder(item, now_ms - 900000, now_ms + 691200000) then
            raise exception 'invalid push reminder' using errcode = '22023';
        end if;
    end loop;
    -- The endpoint lock serializes replacement on one device.
    insert into public.werdy_push_subscriptions(owner, endpoint, p256dh, auth)
        values(caller, subscription->>'endpoint', subscription->>'p256dh', subscription->>'auth')
        on conflict (endpoint) do update set owner = excluded.owner,
            p256dh = excluded.p256dh, auth = excluded.auth
        returning id into subscription_id;
    delete from public.werdy_push_reminders r where r.subscription = subscription_id;
    for item in select pg_catalog.jsonb_array_elements(reminders) loop
        insert into public.werdy_push_reminders(subscription, reminder, send_at, title, body, topic)
            values(subscription_id, item->>'id',
                pg_catalog.to_timestamp((item->>'send_at_ms')::double precision / 1000.0),
                item->>'title', item->>'body', item->>'topic');
    end loop;
end $$;

create or replace function public.werdy_push_forget(endpoint text)
returns void language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := auth.uid();
begin
    if caller is null then raise exception 'sign in required' using errcode = '42501'; end if;
    delete from public.werdy_push_subscriptions s where s.owner = caller and s.endpoint = $1;
end $$;

create or replace function public.werdy_push_claim_due()
returns table(subscription uuid, reminder text, send_at timestamptz, title text, body text,
    topic text, endpoint text, p256dh text, auth text)
language plpgsql volatile security definer set search_path = '' as $$
declare cutoff timestamptz := pg_catalog.clock_timestamp();
begin
    delete from public.werdy_push_reminders r where r.send_at < cutoff - interval '15 minutes';
    return query
        delete from public.werdy_push_reminders r using public.werdy_push_subscriptions s
        where r.subscription = s.id and r.send_at <= cutoff
        returning r.subscription, r.reminder, r.send_at, r.title, r.body, r.topic,
            s.endpoint, s.p256dh, s.auth;
end $$;

create or replace function public.werdy_push_drop_subscription(id uuid)
returns void language plpgsql volatile security definer set search_path = '' as $$
begin
    delete from public.werdy_push_subscriptions s where s.id = $1;
end $$;

-- Account deletion lists tables so the new rows are removed there too.
create or replace function public.werdy_account_delete()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    delete from public.werdy_push_reminders r using public.werdy_push_subscriptions s
        where r.subscription = s.id and s.owner = caller;
    delete from public.werdy_push_subscriptions s where s.owner = caller;
    delete from werdy_private.share_codes c where c.owner = caller;
    delete from werdy_private.share_attempts a where a.watcher = caller;
    delete from public.werdy_shares s where s.owner = caller or s.watcher = caller;
    delete from public.werdy_sync_changes c where c.owner = caller;
    delete from public.werdy_sync_heads h where h.owner = caller;
    delete from auth.users u where u.id = caller;
    if not found then raise exception 'account not found' using errcode = '42501'; end if;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

revoke all on function werdy_private.push_valid_subscription(jsonb),
    werdy_private.push_valid_reminder(jsonb,numeric,numeric) from public, anon, authenticated;
revoke all on function public.werdy_push_replace(jsonb,jsonb), public.werdy_push_forget(text),
    public.werdy_push_claim_due(), public.werdy_push_drop_subscription(uuid)
    from public, anon, authenticated;
grant execute on function public.werdy_push_replace(jsonb,jsonb), public.werdy_push_forget(text)
    to authenticated;
grant execute on function public.werdy_push_claim_due(), public.werdy_push_drop_subscription(uuid)
    to service_role;
notify pgrst, 'reload schema';
commit;
