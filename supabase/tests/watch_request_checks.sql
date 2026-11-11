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
    begin perform public.werdy_request_send(t.receiver::text || '@requests.test','invalid'); exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'invalid direction accepted'; end if;
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
-- Revoke ends a waiting request too: a revoked watcher cannot accept an older request to get back in.
create temporary table revoke_test(owner uuid, watcher uuid, request_id uuid);
insert into revoke_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null);
grant select, update on revoke_test to authenticated;
insert into auth.users(id,email) select owner,owner::text || '@revoke.test' from revoke_test
    union all select watcher,watcher::text || '@revoke.test' from revoke_test;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from revoke_test;
set local role authenticated;
select public.werdy_request_send(watcher::text || '@revoke.test','watch_me') from revoke_test;
create temporary table revoke_code as select public.werdy_share_create()->>'code' as code;
grant select on revoke_code to authenticated;
reset role;
update revoke_test t set request_id=r.id from werdy_private.watch_requests r where r.sender=t.owner;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from revoke_test;
set local role authenticated;
do $$
begin
    if not (public.werdy_share_redeem((select code from revoke_code))->>'ok')::boolean then raise exception 'revoke test redeem failed'; end if;
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from revoke_test;
set local role authenticated;
select public.werdy_share_revoke(watcher) from revoke_test;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from revoke_test;
set local role authenticated;
do $$
declare t record; refused boolean := false;
begin
    select * into strict t from revoke_test;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":false}'::jsonb then
        raise exception 'revoked watcher restored access with an old request';
    end if;
    begin perform public.werdy_watch_pull(t.owner,0,1); exception when insufficient_privilege then refused := true; end;
    if not refused then raise exception 'revoked watcher can still read'; end if;
    if public.werdy_request_incoming() is distinct from '[]'::jsonb then raise exception 'revoked request still listed'; end if;
end $$;
reset role;
-- Both directions must preserve the owner's and watcher's consent.
do $$
declare
    test_sender uuid := pg_catalog.gen_random_uuid();
    test_receiver uuid := pg_catalog.gen_random_uuid();
    test_direction text;
    incoming jsonb;
    rejected boolean;
    expected_owner uuid;
    expected_watcher uuid;
begin
    insert into auth.users(id,email) values(test_sender, test_sender::text || '@directions.test'),
        (test_receiver, test_receiver::text || '@directions.test');
    foreach test_direction in array array['watch_me', 'watch_them'] loop
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        set local role authenticated;
        if public.werdy_request_send(test_receiver::text || '@directions.test',test_direction) is distinct from '{"ok":true}'::jsonb then
            raise exception 'direction send failed: %',test_direction;
        end if;
        if public.werdy_request_send(pg_catalog.gen_random_uuid()::text || '@unknown.test',test_direction) is distinct from '{"ok":true}'::jsonb then
            raise exception 'direction send reveals account existence: %',test_direction;
        end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        incoming := public.werdy_request_incoming();
        if pg_catalog.jsonb_array_length(incoming) <> 1 or incoming#>>'{0,direction}' is distinct from test_direction then
            raise exception 'incoming direction wrong: %',test_direction;
        end if;
        if public.werdy_request_accept((incoming#>>'{0,id}')::uuid) is distinct from '{"ok":true}'::jsonb then
            raise exception 'direction accept failed: %',test_direction;
        end if;
        reset role;
        expected_owner := case when test_direction = 'watch_me' then test_sender else test_receiver end;
        expected_watcher := case when test_direction = 'watch_me' then test_receiver else test_sender end;
        if not exists(select 1 from public.werdy_shares s where s.owner = expected_owner and s.watcher = expected_watcher)
            or exists(select 1 from public.werdy_shares s where s.owner = expected_watcher and s.watcher = expected_owner) then
            raise exception 'accepted share direction wrong: %',test_direction;
        end if;
        if exists(select 1 from werdy_private.watch_requests r where r.sender = test_sender and r.receiver_email = test_receiver::text || '@directions.test') then
            raise exception 'accepted direction request remains';
        end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        set local role authenticated;
        rejected := false;
        begin perform public.werdy_request_send(test_receiver::text || '@directions.test',test_direction);
            exception when invalid_parameter_value then rejected := true; end;
        if not rejected then raise exception 'existing share accepted for sender: %',test_direction; end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        rejected := false;
        begin perform public.werdy_request_send(test_sender::text || '@directions.test',
            case when test_direction = 'watch_me' then 'watch_them' else 'watch_me' end);
            exception when invalid_parameter_value then rejected := true; end;
        if not rejected then raise exception 'existing share accepted for receiver: %',test_direction; end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        perform public.werdy_request_send(test_receiver::text || '@directions.test',
            case when test_direction = 'watch_me' then 'watch_them' else 'watch_me' end);
        reset role;
        if not exists(select 1 from werdy_private.watch_requests r where r.sender = test_sender and r.receiver_email = test_receiver::text || '@directions.test') then
            raise exception 'opposite share request blocked: %',test_direction;
        end if;
        delete from public.werdy_shares s where s.owner in (test_sender, test_receiver);
        delete from werdy_private.watch_requests r where r.sender = test_sender;
    end loop;
end $$;

do $$
declare
    test_sender uuid := pg_catalog.gen_random_uuid();
    test_receiver uuid := pg_catalog.gen_random_uuid();
    test_direction text;
    expected_owner uuid;
    expected_watcher uuid;
begin
    insert into auth.users(id,email) values(test_sender, pg_catalog.upper(test_sender::text || '@crossing.test')),
        (test_receiver, pg_catalog.upper(test_receiver::text || '@crossing.test'));
    foreach test_direction in array array['watch_me', 'watch_them'] loop
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        set local role authenticated;
        perform public.werdy_request_send(test_receiver::text || '@crossing.test',test_direction);
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        if public.werdy_request_send('  ' || pg_catalog.upper(test_sender::text || '@crossing.test') || '  ',
            case when test_direction = 'watch_me' then 'watch_them' else 'watch_me' end) is distinct from '{"ok":true}'::jsonb then
            raise exception 'crossing reply differs: %',test_direction;
        end if;
        reset role;
        expected_owner := case when test_direction = 'watch_me' then test_sender else test_receiver end;
        expected_watcher := case when test_direction = 'watch_me' then test_receiver else test_sender end;
        if (select pg_catalog.count(*) from public.werdy_shares s where s.owner in (test_sender, test_receiver)) <> 1
            or not exists(select 1 from public.werdy_shares s where s.owner = expected_owner and s.watcher = expected_watcher) then
            raise exception 'crossing share direction wrong: %',test_direction;
        end if;
        if exists(select 1 from werdy_private.watch_requests r where r.sender in (test_sender, test_receiver)) then
            raise exception 'crossing left requests: %',test_direction;
        end if;
        delete from public.werdy_shares s where s.owner in (test_sender, test_receiver);
    end loop;
end $$;

do $$
declare
    test_sender uuid := pg_catalog.gen_random_uuid();
    test_receiver uuid := pg_catalog.gen_random_uuid();
    test_direction text;
begin
    insert into auth.users(id,email) values(test_sender, test_sender::text || '@waiting.test'),
        (test_receiver, test_receiver::text || '@waiting.test');
    foreach test_direction in array array['watch_me', 'watch_them'] loop
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        set local role authenticated;
        perform public.werdy_request_send(test_receiver::text || '@waiting.test',test_direction);
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        perform public.werdy_request_send(test_sender::text || '@waiting.test',test_direction);
        reset role;
        if exists(select 1 from public.werdy_shares s where s.owner in (test_sender, test_receiver))
            or (select pg_catalog.count(*) from werdy_private.watch_requests r where r.sender in (test_sender, test_receiver)) <> 2 then
            raise exception 'different shares crossed: %',test_direction;
        end if;
        delete from werdy_private.watch_requests r where r.sender in (test_sender, test_receiver);
        insert into werdy_private.watch_requests(sender, receiver_email, direction, created_at, expires_at)
            values(test_sender, test_receiver::text || '@waiting.test', test_direction, 0, 0);
        set local role authenticated;
        perform public.werdy_request_send(test_sender::text || '@waiting.test',
            case when test_direction = 'watch_me' then 'watch_them' else 'watch_me' end);
        reset role;
        if exists(select 1 from public.werdy_shares s where s.owner in (test_sender, test_receiver))
            or (select pg_catalog.count(*) from werdy_private.watch_requests r where r.sender in (test_sender, test_receiver)) <> 1 then
            raise exception 'expired request crossed: %',test_direction;
        end if;
        delete from werdy_private.watch_requests r where r.sender in (test_sender, test_receiver);
    end loop;
end $$;
-- Sender controls, quiet removal, expiry and the shared limit in both directions.
do $$
declare
    test_sender uuid := pg_catalog.gen_random_uuid();
    test_receiver uuid := pg_catalog.gen_random_uuid();
    test_third uuid := pg_catalog.gen_random_uuid();
    outgoing jsonb;
    request_id uuid;
    test_direction text;
    i integer;
    rejected boolean;
begin
    insert into auth.users(id,email) values(test_sender, test_sender::text || '@outgoing.test'),
        (test_receiver, test_receiver::text || '@outgoing.test'), (test_third, test_third::text || '@outgoing.test');
    foreach test_direction in array array['watch_me', 'watch_them'] loop
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        set local role authenticated;
        perform public.werdy_request_send('  ' || pg_catalog.upper(test_receiver::text || '@outgoing.test') || '  ',test_direction);
        outgoing := public.werdy_request_outgoing();
        if pg_catalog.jsonb_array_length(outgoing) <> 1
            or outgoing#>>'{0,email}' is distinct from test_receiver::text || '@outgoing.test'
            or outgoing#>>'{0,direction}' is distinct from test_direction
            or (outgoing#>>'{0,expires_at}')::bigint is distinct from (outgoing#>>'{0,created_at}')::bigint + 2592000000 then
            raise exception 'outgoing contract wrong';
        end if;
        request_id := (outgoing#>>'{0,id}')::uuid;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_third)::text,true);
        if public.werdy_request_outgoing() is distinct from '[]'::jsonb then raise exception 'third sees outgoing'; end if;
        if public.werdy_request_cancel(request_id) is distinct from '{"ok":true}'::jsonb then raise exception 'third cancel reply wrong'; end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        if public.werdy_request_outgoing() is distinct from outgoing then raise exception 'third cancelled request'; end if;
        if public.werdy_request_cancel(request_id) is distinct from '{"ok":true}'::jsonb
            or public.werdy_request_cancel(request_id) is distinct from '{"ok":true}'::jsonb
            or public.werdy_request_outgoing() is distinct from '[]'::jsonb then raise exception 'sender cancel failed'; end if;
        perform public.werdy_request_send(test_receiver::text || '@outgoing.test',test_direction);
        outgoing := public.werdy_request_outgoing();
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        perform public.werdy_request_decline((outgoing#>>'{0,id}')::uuid);
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        if public.werdy_request_outgoing() is distinct from '[]'::jsonb then raise exception 'decline left outgoing trace'; end if;
        reset role;
        insert into werdy_private.watch_requests(sender, receiver_email, direction, created_at, expires_at)
            values(test_sender, test_receiver::text || '@outgoing.test',test_direction,0,0);
        set local role authenticated;
        if public.werdy_request_outgoing() is distinct from '[]'::jsonb then raise exception 'expired outgoing visible'; end if;
        reset role;
        if exists(select 1 from werdy_private.watch_requests r where r.sender = test_sender) then raise exception 'expired outgoing not deleted'; end if;
        -- Send must clear expiry before counting, even without a preceding list.
        insert into werdy_private.watch_requests(sender, receiver_email, direction, created_at, expires_at)
            values(test_sender, test_receiver::text || '@outgoing.test',test_direction,0,0);
        set local role authenticated;
        for i in 1..5 loop
            perform public.werdy_request_send(i::text || '@limit.test',
                case when i % 2 = 0 then 'watch_me' else 'watch_them' end);
        end loop;
        outgoing := public.werdy_request_outgoing();
        if pg_catalog.jsonb_array_length(outgoing) <> 5 then raise exception 'five sends refused'; end if;
        rejected := false;
        begin
            perform public.werdy_request_send('sixth@limit.test',test_direction);
        exception when sqlstate 'P0001' then
            if sqlerrm <> 'watch request limit reached' then raise; end if;
            rejected := true;
        end;
        if not rejected then raise exception 'sixth send accepted'; end if;
        -- Duplicate errors keep their original SQLSTATE even at the limit.
        rejected := false;
        begin perform public.werdy_request_send('1@limit.test','watch_them');
            exception when invalid_parameter_value then rejected := true; end;
        if not rejected then raise exception 'duplicate at limit error wrong'; end if;
        perform public.werdy_request_cancel((outgoing#>>'{0,id}')::uuid);
        perform public.werdy_request_send('sixth@limit.test',test_direction);
        if pg_catalog.jsonb_array_length(public.werdy_request_outgoing()) <> 5 then raise exception 'cancel did not free slot'; end if;
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_receiver)::text,true);
        perform public.werdy_request_send(test_sender::text || '@outgoing.test',
            case when test_direction = 'watch_me' then 'watch_them' else 'watch_me' end);
        perform pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',test_sender)::text,true);
        if public.werdy_request_send(test_receiver::text || '@outgoing.test',test_direction) is distinct from '{"ok":true}'::jsonb then
            raise exception 'crossing at limit refused';
        end if;
        if pg_catalog.jsonb_array_length(public.werdy_request_outgoing()) <> 5 then raise exception 'crossing inserted request'; end if;
        reset role;
        if not exists(select 1 from public.werdy_shares s
            where s.owner = case when test_direction = 'watch_me' then test_sender else test_receiver end
            and s.watcher = case when test_direction = 'watch_me' then test_receiver else test_sender end) then
            raise exception 'crossing at limit did not create share';
        end if;
        delete from public.werdy_shares s where s.owner in (test_sender,test_receiver);
        delete from werdy_private.watch_requests r where r.sender in (test_sender,test_receiver);
    end loop;
end $$;
-- A share made by crossing requests is ended by revoke like any other share.
create temporary table cross_revoke(owner uuid, watcher uuid);
insert into cross_revoke values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid());
grant select on cross_revoke to authenticated;
insert into auth.users(id,email) select owner,owner::text || '@cross.test' from cross_revoke
    union all select watcher,watcher::text || '@cross.test' from cross_revoke;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from cross_revoke;
set local role authenticated;
select public.werdy_request_send(owner::text || '@cross.test','watch_them') from cross_revoke;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from cross_revoke;
set local role authenticated;
select public.werdy_request_send(watcher::text || '@cross.test','watch_me') from cross_revoke;
select public.werdy_share_revoke(watcher) from cross_revoke;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from cross_revoke;
set local role authenticated;
do $$
declare t record; refused boolean := false;
begin
    select * into strict t from cross_revoke;
    begin perform public.werdy_watch_pull(t.owner,0,1); exception when insufficient_privilege then refused := true; end;
    if not refused then raise exception 'crossing share survived revoke'; end if;
end $$;
reset role;
do $$
begin
    if exists(select 1 from public.werdy_shares s join cross_revoke t on s.owner=t.owner and s.watcher=t.watcher)
        or exists(select 1 from werdy_private.watch_requests r join cross_revoke t on r.sender in (t.owner,t.watcher)) then
        raise exception 'revoke left a crossing share or request';
    end if;
end $$;
-- The pair lock key ignores letter case and argument order.
do $$
begin
    if werdy_private.watch_pair_key('Z@example.test','a@example.test') is distinct from 'a@example.test:z@example.test'
        or werdy_private.watch_pair_key('z@example.test','A@example.test') is distinct from 'a@example.test:z@example.test'
        or werdy_private.watch_pair_key('a@example.test','z@example.test') is distinct from 'a@example.test:z@example.test' then
        raise exception 'pair lock key depends on case or order';
    end if;
end $$;
-- Leave ends waiting requests too: the owner cannot accept an old request to bring a watcher back.
create temporary table leave_test(owner uuid, watcher uuid, request_id uuid);
insert into leave_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null);
grant select, update on leave_test to authenticated;
insert into auth.users(id,email) select owner,owner::text || '@leave.test' from leave_test
    union all select watcher,watcher::text || '@leave.test' from leave_test;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from leave_test;
set local role authenticated;
select public.werdy_request_send(owner::text || '@leave.test','watch_them') from leave_test;
reset role;
update leave_test t set request_id=r.id from werdy_private.watch_requests r where r.sender=t.watcher;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from leave_test;
set local role authenticated;
create temporary table leave_code as select public.werdy_share_create()->>'code' as code;
grant select on leave_code to authenticated;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',watcher,'role','authenticated')::text,true) from leave_test;
set local role authenticated;
do $$
begin
    if not (public.werdy_share_redeem((select code from leave_code))->>'ok')::boolean then raise exception 'leave test redeem failed'; end if;
end $$;
select public.werdy_share_leave(owner) from leave_test;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',owner,'role','authenticated')::text,true) from leave_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from leave_test;
    if public.werdy_request_accept(t.request_id) is distinct from '{"ok":false}'::jsonb then
        raise exception 'owner restored a watcher who left with an old request';
    end if;
end $$;
reset role;
do $$
begin
    if exists(select 1 from public.werdy_shares s join leave_test t on s.owner=t.owner and s.watcher=t.watcher) then
        raise exception 'share exists after leave';
    end if;
end $$;
rollback;
