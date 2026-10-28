-- Run as database owner after both migrations. Everything rolls back.
begin;
set local request.jwt.claim.sub = '';
create temporary table sharing_test(owner uuid, watcher uuid, third uuid, code text, expired text);
insert into sharing_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null, null);
grant select, update on sharing_test to authenticated;
insert into auth.users(id,email) select owner,'owner@sharing.test' from sharing_test
    union all select watcher,'watcher@sharing.test' from sharing_test
    union all select third,'third@sharing.test' from sharing_test;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from sharing_test;
set local role authenticated;
do $$
declare reply jsonb; i integer; rejected boolean := false;
begin
    reply := public.werdy_share_create();
    if pg_catalog.length(reply->>'code') <> 10 then raise exception 'code shape failed'; end if;
    update sharing_test set code=reply->>'code';
    if (public.werdy_share_redeem(reply->>'code')->>'ok')::boolean then raise exception 'owner redeemed own code'; end if;
    update sharing_test set expired=public.werdy_share_create()->>'code';
    if public.werdy_share_watchers() <> '[]'::jsonb then raise exception 'code granted access'; end if;
    perform public.werdy_sync_push('[{"install":"sharing_test","operation":1,"change":{"Note":{"day":20754,"note":{"text":"late and excused","credit":"half","entered_on":20758}}}}]');
    for i in 1..3 loop perform public.werdy_share_create(); end loop;
    begin perform public.werdy_share_create(); exception when invalid_parameter_value then rejected:=true; end;
    if not rejected then raise exception 'active code limit failed'; end if;
end $$;
reset role;
-- Only the test administrator can expire a code or see its stored hash.
update werdy_private.share_codes c set expires_at=0 from sharing_test t
    where c.hash=pg_catalog.sha256(pg_catalog.convert_to(t.expired,'UTF8'));
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from sharing_test;
set local role authenticated;
do $$
declare t record; reply jsonb; rejected boolean;
begin
    select * into strict t from sharing_test;
    if (public.werdy_share_redeem(t.expired)->>'ok')::boolean then raise exception 'expired code accepted'; end if;
    if not (public.werdy_share_redeem(t.code)->>'ok')::boolean then raise exception 'redemption failed'; end if;
    if (public.werdy_share_redeem(t.code)->>'ok')::boolean then raise exception 'used code accepted'; end if;
    reply := public.werdy_watch_pull(t.owner,0,1);
    if reply#>>'{changes,0,pending,change,Note,note,text}' <> 'late and excused'
        or reply#>>'{changes,0,pending,change,Note,note,credit}' <> 'half'
        or reply#>>'{changes,0,pending,change,Note,note,entered_on}' <> '20758' then
        raise exception 'watched note or signs missing';
    end if;
    if pg_catalog.jsonb_array_length(public.werdy_share_watched()) <> 1 then raise exception 'watched list failed'; end if;
    if exists(select 1 from public.werdy_sync_changes c where c.owner=t.owner) then raise exception 'table read leaked owner'; end if;
    rejected:=false;
    begin insert into public.werdy_sync_changes(owner,sequence,received_at,install,operation,pending)
        values(t.owner,2,0,'intruder',1,'{}'); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'watcher table write allowed'; end if;
    rejected:=false;
    begin perform public.werdy_sync_push('[{"owner":"forged","install":"intruder","operation":1,"change":{"Note":{"day":20754,"note":null}}}]');
        exception when invalid_parameter_value then rejected:=true; end;
    if not rejected then raise exception 'forged owner push accepted'; end if;
    -- A legal push writes only the watcher's own account, never the owner.
    perform public.werdy_sync_push('[{"install":"intruder","operation":1,"change":{"Note":{"day":20754,"note":null}}}]');
    if exists(select 1 from public.werdy_sync_changes c where c.owner<>t.watcher) then raise exception 'sync policy widened'; end if;
    rejected:=false;
    begin perform 1 from public.werdy_shares; exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'direct sharing table access allowed'; end if;
end $$;
reset role;
do $$
begin
    if (select pg_catalog.count(*) from public.werdy_sync_changes c join sharing_test t on c.owner=t.owner) <> 1 then
        raise exception 'watcher changed owner stream';
    end if;
end $$;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',third,'role','authenticated')::text,true) from sharing_test;
set local role authenticated;
do $$
declare t record; rejected boolean:=false; i integer;
begin
    select * into strict t from sharing_test;
    begin perform public.werdy_watch_pull(t.owner,0,500); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'third user read allowed'; end if;
    for i in 1..12 loop
        if (public.werdy_share_redeem('BAD')->>'ok')::boolean then raise exception 'bad code accepted'; end if;
    end loop;
end $$;
reset role;
do $$
begin
    if (select a.failures from werdy_private.share_attempts a join sharing_test t on t.third=a.watcher) <> 10 then
        raise exception 'failed attempt counter rolled back or exceeded limit';
    end if;
end $$;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from sharing_test;
set local role authenticated;
do $$
declare t record; reply jsonb;
begin
    select * into strict t from sharing_test;
    reply:=public.werdy_share_watchers();
    if reply#>>'{0,email}' <> 'watcher@sharing.test' then raise exception 'watcher email list failed'; end if;
    perform public.werdy_share_revoke(t.watcher);
    if public.werdy_share_watchers() <> '[]'::jsonb then raise exception 'revoke list failed'; end if;
    update sharing_test set code=public.werdy_share_create()->>'code';
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from sharing_test;
set local role authenticated;
do $$
declare t record; rejected boolean:=false;
begin
    select * into strict t from sharing_test;
    begin perform public.werdy_watch_pull(t.owner,0,500); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'revoked watcher read allowed'; end if;
    if not (public.werdy_share_redeem(t.code)->>'ok')::boolean then raise exception 'new redemption failed'; end if;
    perform public.werdy_share_leave(t.owner);
    rejected:=false;
    begin perform public.werdy_watch_pull(t.owner,0,500); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'left share still readable'; end if;
end $$;
reset role;
set local role anon;
do $$
declare rejected boolean:=false;
begin
    begin perform public.werdy_share_create(); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'anon create allowed'; end if;
end $$;
reset role;
do $$
declare f record;
begin
    for f in select p.oid from pg_catalog.pg_proc p join pg_catalog.pg_namespace n on n.oid=p.pronamespace
        where n.nspname='public' and (p.proname like 'werdy_share_%' or p.proname='werdy_watch_pull') loop
        if pg_catalog.has_function_privilege('anon',f.oid,'execute') then raise exception 'anon RPC grant'; end if;
    end loop;
end $$;
set local request.jwt.claims = '{}';
set local role authenticated;
do $$
declare rejected boolean:=false;
begin
    begin perform public.werdy_share_create(); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'missing identity accepted'; end if;
end $$;
reset role;
rollback;
