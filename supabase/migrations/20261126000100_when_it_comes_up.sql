-- When it comes up schedules (decision 0047, issue #188). The sync validator
-- now also accepts the plain "when_it_comes_up" schedule. The function is
-- copied whole from 20261126000000_once_a_week.sql with only that line
-- changed, since PL/pgSQL has no way to change one branch. Safe to rerun: it
-- only replaces the function and repeats the same revoke.
begin;

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
        if kind = 'schedule' and v in ('"every_day"'::jsonb, '"once_a_week"'::jsonb,
            '"when_it_comes_up"'::jsonb) then return true; end if;
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

revoke all on function werdy_private.sync_valid(jsonb, text) from public, anon, authenticated;
commit;
