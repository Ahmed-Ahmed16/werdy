-- Run as the database owner. All grants change in the same transaction.
begin;

create schema if not exists werdy_private;
revoke all on schema werdy_private from public, anon, authenticated;

create table if not exists public.werdy_sync_heads (
    owner uuid primary key references auth.users(id) on delete cascade,
    last_sequence bigint not null default 0 check (last_sequence >= 0),
    last_received_at bigint not null default 0 check (last_received_at >= 0)
);

-- The stream is also the permanent receipt ledger. Do not prune it while an
-- account exists. Operation ids cover all Rust u64 values without rounding.
create table if not exists public.werdy_sync_changes (
    owner uuid not null references auth.users(id) on delete cascade,
    sequence bigint not null check (sequence > 0),
    received_at bigint not null check (received_at >= 0),
    install text not null check (install ~ '^[A-Za-z0-9_]{1,128}$'),
    operation numeric(20,0) not null check (operation between 1 and 18446744073709551615),
    pending jsonb not null check (pg_catalog.jsonb_typeof(pending) = 'object'
        and pg_catalog.octet_length(pending::text) <= 16384),
    primary key (owner, sequence),
    unique (owner, install, operation)
);

alter table public.werdy_sync_heads enable row level security;
alter table public.werdy_sync_changes enable row level security;
revoke all on table public.werdy_sync_heads, public.werdy_sync_changes
    from public, anon, authenticated;
grant select on table public.werdy_sync_heads, public.werdy_sync_changes to authenticated;
drop policy if exists werdy_sync_heads_select on public.werdy_sync_heads;
create policy werdy_sync_heads_select on public.werdy_sync_heads
    for select to authenticated using (owner = (select auth.uid()));
drop policy if exists werdy_sync_changes_select on public.werdy_sync_changes;
create policy werdy_sync_changes_select on public.werdy_sync_changes
    for select to authenticated using (owner = (select auth.uid()));

-- Exact keys prevent hidden fields such as location in otherwise valid rows.
create or replace function werdy_private.sync_object(v jsonb, keys text[])
returns boolean language sql immutable set search_path = '' as $$
    select case when pg_catalog.jsonb_typeof(v) = 'object'
        then v ?& keys and v - keys = '{}'::jsonb else false end
$$;

-- This validates the serde wire layout, including nested enum tags and core
-- number ranges. No extension is required. Unknown fields and tags fail closed.
-- Text limits are UTF8 bytes. One operation is at most 16 KiB, a batch at most
-- 1 MiB and 1024 operations. Collections also have explicit length limits.
create or replace function werdy_private.sync_valid(v jsonb, kind text)
returns boolean language plpgsql immutable set search_path = '' as $$
declare
    ok boolean := false;
    tag text;
    body jsonb;
    child jsonb;
    previous_day bigint;
    start_day bigint;
    end_day bigint;
begin
    if v is null then return false; end if;
    case kind
    when 'day', 'u32', 'u64', 'positive_u64', 'u128', 'offset' then
        if pg_catalog.jsonb_typeof(v) <> 'number' or v::text !~ '^-?(0|[1-9][0-9]*)$'
            then return false; end if;
        case kind
        when 'day' then ok := v::text::numeric between -2147483648 and 2147483647;
        when 'u32' then ok := v::text::numeric between 0 and 4294967295;
        when 'u64' then ok := v::text::numeric between 0 and 18446744073709551615;
        when 'positive_u64' then ok := v::text::numeric between 1 and 18446744073709551615;
        when 'u128' then ok := v::text::numeric between 0 and 340282366920938463463374607431768211455;
        when 'offset' then ok := v::text::numeric between -1 and 1;
        end case;
    when 'install', 'key', 'name_text', 'note_text', 'language' then
        if pg_catalog.jsonb_typeof(v) <> 'string' then return false; end if;
        tag := v #>> '{}';
        case kind
        when 'install' then ok := tag ~ '^[A-Za-z0-9_]{1,128}$';
        when 'key' then ok := pg_catalog.octet_length(tag) between 1 and 128;
        when 'name_text' then ok := pg_catalog.octet_length(tag) <= 1024;
        when 'note_text' then ok := pg_catalog.octet_length(tag) <= 8192;
        when 'language' then ok := pg_catalog.octet_length(tag) between 1 and 16;
        end case;
    when 'id' then
        ok := werdy_private.sync_object(v, array['install','sequence'])
            and werdy_private.sync_valid(v->'install', 'install')
            and werdy_private.sync_valid(v->'sequence', 'positive_u64');
    when 'pending' then
        ok := werdy_private.sync_object(v, array['install','operation','change'])
            and werdy_private.sync_valid(v->'install', 'install')
            and werdy_private.sync_valid(v->'operation', 'positive_u64')
            and werdy_private.sync_valid(v->'change', 'change');
    when 'change', 'setting', 'name', 'schedule', 'mark' then
        if kind = 'schedule' and v = '"every_day"'::jsonb then return true; end if;
        if kind = 'mark' and v in ('"done"'::jsonb, '"missed"'::jsonb) then return true; end if;
        if pg_catalog.jsonb_typeof(v) <> 'object' then return false; end if;
        if (select pg_catalog.count(*) from pg_catalog.jsonb_object_keys(v)) <> 1 then return false; end if;
        select entry.key, entry.value into tag, body from pg_catalog.jsonb_each(v) as entry;
        case kind
        when 'change' then
            case tag
            when 'Deed' then ok := werdy_private.sync_object(body, array['id','row'])
                and werdy_private.sync_valid(body->'id','id')
                and (body->'row' = 'null'::jsonb or werdy_private.sync_valid(body->'row','deed'));
            when 'Rule' then ok := werdy_private.sync_object(body, array['id','from_day','rule'])
                and werdy_private.sync_valid(body->'id','id')
                and werdy_private.sync_valid(body->'from_day','day')
                and werdy_private.sync_valid(body->'rule','rule');
            when 'Mark' then ok := werdy_private.sync_object(body, array['id','day','entry'])
                and werdy_private.sync_valid(body->'id','id')
                and werdy_private.sync_valid(body->'day','day')
                and (body->'entry' = 'null'::jsonb or werdy_private.sync_valid(body->'entry','entry'));
            when 'Note' then ok := werdy_private.sync_object(body, array['day','note'])
                and werdy_private.sync_valid(body->'day','day')
                and (body->'note' = 'null'::jsonb or werdy_private.sync_valid(body->'note','note'));
            when 'Setting' then ok := werdy_private.sync_valid(body,'setting');
            when 'ChosenSet' then ok := werdy_private.sync_object(body, array['key','chosen'])
                and werdy_private.sync_valid(body->'key','key')
                and pg_catalog.jsonb_typeof(body->'chosen') = 'boolean';
            else return false;
            end case;
        when 'setting' then
            case tag
            when 'Language' then ok := werdy_private.sync_valid(body,'language');
            when 'Theme' then ok := body in ('"system"'::jsonb, '"light"'::jsonb, '"dark"'::jsonb);
            when 'Digits' then ok := body in ('"western"'::jsonb, '"arabic_indic"'::jsonb);
            when 'Calendar' then ok := werdy_private.sync_valid(body,'calendar');
            else return false;
            end case;
        when 'name' then
            case tag
            when 'key' then ok := werdy_private.sync_valid(body,'key');
            when 'text' then ok := werdy_private.sync_valid(body,'name_text');
            else return false;
            end case;
        when 'schedule' then
            case tag
            when 'weekdays' then
                if pg_catalog.jsonb_typeof(body) <> 'array' then return false; end if;
                if pg_catalog.jsonb_array_length(body) > 7 then return false; end if;
                ok := not exists (select 1 from pg_catalog.jsonb_array_elements(body) as weekday(v)
                    where weekday.v not in ('"monday"'::jsonb, '"tuesday"'::jsonb, '"wednesday"'::jsonb,
                        '"thursday"'::jsonb, '"friday"'::jsonb, '"saturday"'::jsonb, '"sunday"'::jsonb));
                if (select pg_catalog.count(distinct weekday.v) from pg_catalog.jsonb_array_elements(body) as weekday(v))
                    <> pg_catalog.jsonb_array_length(body) then return false; end if;
            when 'hijri' then
                if not werdy_private.sync_object(body, array['month','first_day','last_day']) then return false; end if;
                if not werdy_private.sync_valid(body->'first_day','u32')
                    or not werdy_private.sync_valid(body->'last_day','u32') then return false; end if;
                if body->'month' <> 'null'::jsonb then
                    if not werdy_private.sync_valid(body->'month','u32') then return false; end if;
                    if (body->>'month')::numeric not between 1 and 12 then return false; end if;
                end if;
                ok := (body->>'first_day')::numeric between 1 and (body->>'last_day')::numeric
                    and (body->>'last_day')::numeric <= 30;
            else return false;
            end case;
        when 'mark' then
            case tag
            when 'count' then ok := werdy_private.sync_valid(body,'u32');
            when 'partial' then
                if not werdy_private.sync_object(body,array['numerator','denominator']) then return false; end if;
                if not werdy_private.sync_valid(body->'numerator','u128')
                    or not werdy_private.sync_valid(body->'denominator','u128') then return false; end if;
                ok := (body->>'denominator')::numeric > 0
                    and (body->>'numerator')::numeric <= (body->>'denominator')::numeric;
            else return false;
            end case;
        end case;
    when 'deed' then
        if not werdy_private.sync_object(v,array['created_on','archived_from','past_archives','name','starter_key'])
            or not werdy_private.sync_valid(v->'created_on','day') then return false; end if;
        ok := (v->'name' = 'null'::jsonb or werdy_private.sync_valid(v->'name','name'))
            and (v->'starter_key' = 'null'::jsonb or werdy_private.sync_valid(v->'starter_key','key'));
        if pg_catalog.jsonb_typeof(v->'past_archives') <> 'array' then return false; end if;
        if pg_catalog.jsonb_array_length(v->'past_archives') > 4096 then return false; end if;
        previous_day := (v->>'created_on')::bigint;
        for child in select pg_catalog.jsonb_array_elements(v->'past_archives') loop
            if not werdy_private.sync_valid(child,'day_pair') then return false; end if;
            start_day := (child->>0)::bigint; end_day := (child->>1)::bigint;
            if start_day < previous_day or end_day <= start_day then return false; end if;
            previous_day := end_day;
        end loop;
        if v->'archived_from' <> 'null'::jsonb then
            if not werdy_private.sync_valid(v->'archived_from','day') then return false; end if;
            ok := ok and (v->>'archived_from')::bigint >= previous_day;
        end if;
    when 'day_pair' then
        if pg_catalog.jsonb_typeof(v) <> 'array' then return false; end if;
        ok := pg_catalog.jsonb_array_length(v) = 2
            and werdy_private.sync_valid(v->0,'day') and werdy_private.sync_valid(v->1,'day');
    when 'rule' then
        ok := werdy_private.sync_object(v,array['schedule','target','category','slot'])
            and werdy_private.sync_valid(v->'schedule','schedule')
            and (v->'target' = 'null'::jsonb or werdy_private.sync_valid(v->'target','target'))
            and werdy_private.sync_valid(v->'category','key') and werdy_private.sync_valid(v->'slot','key');
    when 'target' then
        if not werdy_private.sync_object(v,array['amount','unit'])
            or not werdy_private.sync_valid(v->'amount','u32') then return false; end if;
        ok := (v->>'amount')::numeric > 0 and werdy_private.sync_valid(v->'unit','key');
    when 'entry' then
        ok := werdy_private.sync_object(v,array['mark','entered_on'])
            and werdy_private.sync_valid(v->'mark','mark') and werdy_private.sync_valid(v->'entered_on','day');
    when 'note' then
        ok := werdy_private.sync_object(v,array['text','credit','entered_on'])
            and werdy_private.sync_valid(v->'text','note_text')
            and v->'credit' in ('"none"'::jsonb, '"half"'::jsonb, '"full"'::jsonb)
            and werdy_private.sync_valid(v->'entered_on','day')
            and (v->>'text' <> '' or v->>'credit' <> 'none');
    when 'calendar' then
        if not werdy_private.sync_object(v,array['first_offset','later_offsets'])
            or not werdy_private.sync_valid(v->'first_offset','offset') then return false; end if;
        if pg_catalog.jsonb_typeof(v->'later_offsets') <> 'array' then return false; end if;
        if pg_catalog.jsonb_array_length(v->'later_offsets') > 4096 then return false; end if;
        previous_day := -2147483649;
        for child in select pg_catalog.jsonb_array_elements(v->'later_offsets') loop
            if pg_catalog.jsonb_typeof(child) <> 'array' then return false; end if;
            if pg_catalog.jsonb_array_length(child) <> 2
                or not werdy_private.sync_valid(child->0,'day')
                or not werdy_private.sync_valid(child->1,'offset') then return false; end if;
            start_day := (child->>0)::bigint;
            if start_day <= previous_day then return false; end if;
            previous_day := start_day;
        end loop;
        ok := true;
    else return false;
    end case;
    return coalesce(ok, false);
end
$$;

create or replace function public.werdy_sync_push(batch jsonb)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    owner_id uuid := auth.uid();
    head public.werdy_sync_heads%rowtype;
    receipt public.werdy_sync_changes%rowtype;
    operation_json jsonb;
    replies jsonb := '[]'::jsonb;
begin
    if owner_id is null then raise exception 'sign in required' using errcode = '42501'; end if;
    if batch is null or pg_catalog.jsonb_typeof(batch) <> 'array' then
        raise exception 'batch must be an array' using errcode = '22023';
    end if;
    if pg_catalog.jsonb_array_length(batch) > 1024 or pg_catalog.octet_length(batch::text) > 1048576 then
        raise exception 'batch exceeds sync limits' using errcode = '22023';
    end if;
    -- Validate before obtaining the lock. Any later failure rolls back the batch.
    for operation_json in select pg_catalog.jsonb_array_elements(batch) loop
        if pg_catalog.octet_length(operation_json::text) > 16384 or not werdy_private.sync_valid(operation_json,'pending') then
            raise exception 'invalid sync operation' using errcode = '22023';
        end if;
    end loop;
    insert into public.werdy_sync_heads(owner) values (owner_id) on conflict (owner) do nothing;
    select h.* into strict head from public.werdy_sync_heads as h where h.owner = owner_id for update;
    for operation_json in select pg_catalog.jsonb_array_elements(batch) loop
        select c.* into receipt from public.werdy_sync_changes as c
            where c.owner = owner_id and c.install = operation_json->>'install'
                and c.operation = (operation_json->>'operation')::numeric;
        if found then
            if receipt.pending <> operation_json then
                raise exception 'operation id reused for different change' using errcode = '22023';
            end if;
        else
            if head.last_sequence = 9223372036854775807 then
                raise exception 'sync sequence exhausted' using errcode = '22003';
            end if;
            head.last_sequence := head.last_sequence + 1;
            head.last_received_at := greatest(head.last_received_at,
                pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint);
            insert into public.werdy_sync_changes(owner, sequence, received_at, install, operation, pending)
                values (owner_id, head.last_sequence, head.last_received_at, operation_json->>'install',
                    (operation_json->>'operation')::numeric, operation_json) returning * into receipt;
        end if;
        replies := replies || pg_catalog.jsonb_build_array(pg_catalog.jsonb_build_object(
            'pending', receipt.pending, 'stamp', pg_catalog.jsonb_build_object(
                'received_at', receipt.received_at, 'sequence', receipt.sequence)));
    end loop;
    update public.werdy_sync_heads as h set last_sequence = head.last_sequence,
        last_received_at = head.last_received_at where h.owner = owner_id;
    return pg_catalog.jsonb_build_object('changes', replies, 'cursor', head.last_sequence);
end
$$;

-- Returning a scalar JSON value avoids the PostgREST row limit. The captured
-- head bounds this read, so a concurrent push cannot make the response partial.
create or replace function public.werdy_sync_pull(after_cursor numeric)
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare
    owner_id uuid := auth.uid();
    cursor bigint;
    changes jsonb;
begin
    if owner_id is null then raise exception 'sign in required' using errcode = '42501'; end if;
    if after_cursor is null or after_cursor < 0 or after_cursor > 9223372036854775807
        or after_cursor <> pg_catalog.trunc(after_cursor) then
        raise exception 'invalid sync cursor' using errcode = '22023';
    end if;
    select h.last_sequence into cursor from public.werdy_sync_heads as h where h.owner = owner_id;
    cursor := coalesce(cursor, 0);
    if after_cursor > cursor then raise exception 'cursor is ahead of server' using errcode = '22023'; end if;
    select coalesce(pg_catalog.jsonb_agg(pg_catalog.jsonb_build_object(
        'pending', c.pending, 'stamp', pg_catalog.jsonb_build_object(
            'received_at', c.received_at, 'sequence', c.sequence)) order by c.sequence), '[]'::jsonb)
        into changes from public.werdy_sync_changes as c
        where c.owner = owner_id and c.sequence > after_cursor and c.sequence <= cursor;
    return pg_catalog.jsonb_build_object('changes', changes, 'cursor', cursor);
end
$$;

create or replace function public.werdy_sync_server_time()
returns bigint language plpgsql volatile security definer set search_path = '' as $$
begin
    if auth.uid() is null then raise exception 'sign in required' using errcode = '42501'; end if;
    return pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint;
end
$$;

revoke all on all functions in schema werdy_private from public, anon, authenticated;
revoke all on function public.werdy_sync_push(jsonb), public.werdy_sync_pull(numeric),
    public.werdy_sync_server_time() from public, anon, authenticated;
grant execute on function public.werdy_sync_push(jsonb), public.werdy_sync_pull(numeric),
    public.werdy_sync_server_time() to authenticated;
notify pgrst, 'reload schema';
commit;
