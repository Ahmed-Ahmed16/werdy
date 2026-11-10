-- Run as database owner after the watch requests migration. Everything rolls back.
begin;
set local request.jwt.claim.sub = '';
create temporary table request_test(sender uuid, receiver uuid, third uuid, request_id uuid);
insert into request_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null);
grant select, update on request_test to authenticated;
insert into auth.users(id,email) select sender,sender::text || '@requests.test' from request_test
    union all select receiver,receiver::text || '@requests.test' from request_test
    union all select third,third::text || '@requests.test' from request_test;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',sender,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record; reply jsonb; rejected boolean; bad text;
begin
    select * into strict t from request_test;
    reply := public.werdy_request_send('  ' || pg_catalog.upper(t.receiver::text || '@requests.test') || '  ', 'watch_me');
    if reply is distinct from '{"ok":true}'::jsonb then raise exception 'send failed'; end if;
    if public.werdy_request_send(pg_catalog.gen_random_uuid()::text || '@unknown.test','watch_me') is distinct from reply then
        raise exception 'send reveals account existence';
    end if;
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'sender sees request'; end if;
    foreach bad in array array[t.sender::text || '@requests.test', t.receiver::text || '@requests.test', '', 'a', '@b', 'a@', 'a@@b', 'a @b', pg_catalog.repeat('a',253) || '@b'] loop
        rejected := false;
        begin perform public.werdy_request_send(bad,'watch_me'); exception when invalid_parameter_value then rejected := true; end;
        if not rejected then raise exception 'invalid or duplicate email accepted: %',bad; end if;
    end loop;
    rejected := false;
    begin perform public.werdy_request_send(t.receiver::text || '@requests.test','watch_them'); exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'watch_them accepted'; end if;
    rejected := false;
    begin perform 1 from werdy_private.watch_requests; exception when insufficient_privilege then rejected := true; end;
    if not rejected then raise exception 'direct table select allowed'; end if;
end $$;
reset role;
do $$
begin
    if exists(select 1 from werdy_private.watch_requests r join request_test t on r.sender=t.sender
        where r.expires_at is distinct from r.created_at + 2592000000) then raise exception 'expiry duration wrong'; end if;
end $$;
update request_test t set request_id=r.id from werdy_private.watch_requests r
    where r.sender=t.sender and r.receiver_email=t.receiver::text || '@requests.test';
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',third,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from request_test;
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'third sees request'; end if;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":false}'::jsonb then raise exception 'third accepted'; end if;
    if public.werdy_request_decline(t.request_id) is distinct from '{"ok":true}'::jsonb then raise exception 'decline reply failed'; end if;
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',receiver,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record; incoming jsonb;
begin
    select * into strict t from request_test;
    incoming := public.werdy_request_incoming();
    if pg_catalog.jsonb_array_length(incoming) is distinct from 1 or incoming#>>'{0,id}' is distinct from t.request_id::text
        or incoming#>>'{0,email}' is distinct from t.sender::text || '@requests.test'
        or incoming#>>'{0,direction}' is distinct from 'watch_me' then raise exception 'incoming mismatch or third declined'; end if;
    perform public.werdy_request_decline(t.request_id);
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'decline did not delete'; end if;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":false}'::jsonb then raise exception 'declined accepted'; end if;
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',sender,'role','authenticated')::text,true) from request_test;
set local role authenticated;
select public.werdy_request_send(receiver::text || '@requests.test','watch_me') from request_test;
reset role;
update request_test t set request_id=r.id from werdy_private.watch_requests r
    where r.sender=t.sender and r.receiver_email=t.receiver::text || '@requests.test';
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',receiver,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from request_test;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":true}'::jsonb then raise exception 'accept failed'; end if;
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'accepted request remains'; end if;
    if public.werdy_share_watched()#>>'{0,id}' is distinct from t.sender::text then raise exception 'watched owner missing'; end if;
end $$;
reset role;
do $$
begin
    if not exists(select 1 from public.werdy_shares s join request_test t on s.owner=t.sender and s.watcher=t.receiver) then
        raise exception 'share direction wrong';
    end if;
end $$;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',sender,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record; rejected boolean := false;
begin
    select * into strict t from request_test;
    begin perform public.werdy_request_send(t.receiver::text || '@requests.test','watch_me'); exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'existing share accepted'; end if;
    perform public.werdy_share_revoke(t.receiver);
    perform public.werdy_request_send(t.receiver::text || '@requests.test','watch_me');
end $$;
reset role;
update request_test t set request_id=r.id from werdy_private.watch_requests r
    where r.sender=t.sender and r.receiver_email=t.receiver::text || '@requests.test';
update werdy_private.watch_requests r set expires_at=0 from request_test t where r.id=t.request_id;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',receiver,'role','authenticated')::text,true) from request_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from request_test;
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'expired visible'; end if;
    perform public.werdy_request_send(t.third::text || '@requests.test','watch_me');
end $$;
reset role;
-- Recreate an expired row so accept is checked without a preceding list call.
insert into werdy_private.watch_requests(id,sender,receiver_email,direction,created_at,expires_at)
    select request_id,sender,receiver::text || '@requests.test','watch_me',0,0 from request_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from request_test;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":false}'::jsonb then raise exception 'expired accepted'; end if;
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',sender,'role','authenticated')::text,true) from request_test;
set local role authenticated;
select public.werdy_request_send(receiver::text || '@requests.test','watch_me') from request_test;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',receiver,'role','authenticated')::text,true) from request_test;
set local role authenticated;
select public.werdy_account_delete();
reset role;
do $$
begin
    if exists(select 1 from werdy_private.watch_requests r join request_test t
        on r.sender=t.receiver or r.receiver_email=t.receiver::text || '@requests.test') then
        raise exception 'account deletion left sent or addressed request';
    end if;
    if exists(select 1 from auth.users u join request_test t on u.id=t.receiver) then raise exception 'account remains'; end if;
end $$;
rollback;
