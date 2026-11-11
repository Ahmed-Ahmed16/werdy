-- Run as database owner after all four migrations. Everything rolls back.
begin;
set local request.jwt.claim.sub = '';
create temporary table delete_test(gone uuid, kept uuid, code text, gone_push uuid, kept_push uuid);
insert into delete_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null, null, null);
grant select, update on delete_test to authenticated;
insert into auth.users(id,email) select gone,'gone@delete.test' from delete_test
    union all select kept,'kept@delete.test' from delete_test;

-- The kept user owns data and shares with the user who will leave.
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',kept,'role','authenticated')::text,true) from delete_test;
set local role authenticated;
do $$
declare now_ms bigint := pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint;
begin
    perform public.werdy_sync_push('[{"install":"delete_test","operation":1,"change":{"Note":{"day":20754,"note":{"text":"kept note","credit":"none","entered_on":20754}}}}]');
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/kept","p256dh":"kept_key","auth":"kept_auth"}',
        pg_catalog.jsonb_build_array(pg_catalog.jsonb_build_object('id','kept','send_at_ms',now_ms + 60000,
            'title','Kept','body','Kept reminder','topic','kept_1')));
    update delete_test set kept_push = (select id from public.werdy_push_subscriptions
        where endpoint = 'https://push.example.test/kept');
    update delete_test set code=public.werdy_share_create()->>'code';
end $$;
reset role;

-- The leaving user has data, an open code, a failed attempt and shares both ways.
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',gone,'role','authenticated')::text,true) from delete_test;
set local role authenticated;
do $$
declare t record; own_code text;
    now_ms bigint := pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint;
begin
    select * into strict t from delete_test;
    perform public.werdy_sync_push('[{"install":"delete_test","operation":1,"change":{"Note":{"day":20754,"note":{"text":"gone note","credit":"none","entered_on":20754}}}}]');
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/gone","p256dh":"gone_key","auth":"gone_auth"}',
        pg_catalog.jsonb_build_array(pg_catalog.jsonb_build_object('id','gone','send_at_ms',now_ms + 60000,
            'title','Gone','body','Gone reminder','topic','gone_1')));
    update delete_test set gone_push = (select id from public.werdy_push_subscriptions
        where endpoint = 'https://push.example.test/gone');
    if not (public.werdy_share_redeem(t.code)->>'ok')::boolean then raise exception 'setup redeem failed'; end if;
    if (public.werdy_share_redeem('AAAAAAAAAA')->>'ok')::boolean then raise exception 'setup bad code accepted'; end if;
    own_code := public.werdy_share_create()->>'code';
    perform public.werdy_share_create();
    update delete_test set code=own_code;
end $$;
reset role;
select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',kept,'role','authenticated')::text,true) from delete_test;
set local role authenticated;
do $$
declare t record;
begin
    select * into strict t from delete_test;
    if not (public.werdy_share_redeem(t.code)->>'ok')::boolean then raise exception 'setup reverse redeem failed'; end if;
end $$;
reset role;

select pg_catalog.set_config('request.jwt.claims', pg_catalog.jsonb_build_object('sub',gone,'role','authenticated')::text,true) from delete_test;
set local role authenticated;
do $$
begin
    if not (public.werdy_account_delete()->>'ok')::boolean then raise exception 'delete failed'; end if;
end $$;
-- A second call after deletion is refused, not silently accepted.
do $$
declare rejected boolean := false;
begin
    begin perform public.werdy_account_delete(); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'second delete accepted'; end if;
end $$;
reset role;

do $$
declare t record;
begin
    select * into strict t from delete_test;
    if exists(select 1 from auth.users u where u.id=t.gone) then raise exception 'auth user kept'; end if;
    if exists(select 1 from public.werdy_sync_changes c where c.owner=t.gone) then raise exception 'changes kept'; end if;
    if exists(select 1 from public.werdy_sync_heads h where h.owner=t.gone) then raise exception 'head kept'; end if;
    if exists(select 1 from public.werdy_shares s where s.owner=t.gone or s.watcher=t.gone) then raise exception 'shares kept'; end if;
    if exists(select 1 from werdy_private.share_codes c where c.owner=t.gone) then raise exception 'codes kept'; end if;
    if exists(select 1 from werdy_private.share_attempts a where a.watcher=t.gone) then raise exception 'attempts kept'; end if;
    if exists(select 1 from public.werdy_push_subscriptions s where s.id=t.gone_push) then raise exception 'push subscription kept'; end if;
    if exists(select 1 from public.werdy_push_reminders r where r.subscription=t.gone_push) then raise exception 'push reminders kept'; end if;
    if not exists(select 1 from auth.users u where u.id=t.kept) then raise exception 'other user deleted'; end if;
    if not exists(select 1 from public.werdy_sync_changes c where c.owner=t.kept) then raise exception 'other changes deleted'; end if;
    if not exists(select 1 from werdy_private.share_codes c where c.owner=t.kept) then raise exception 'other codes deleted'; end if;
    if not exists(select 1 from public.werdy_push_subscriptions s where s.id=t.kept_push) then raise exception 'other push subscription deleted'; end if;
    if not exists(select 1 from public.werdy_push_reminders r where r.subscription=t.kept_push) then raise exception 'other push reminder deleted'; end if;
end $$;

set local role anon;
do $$
declare rejected boolean := false;
begin
    begin perform public.werdy_account_delete(); exception when insufficient_privilege then rejected:=true; end;
    if not rejected then raise exception 'anon delete allowed'; end if;
end $$;
reset role;
do $$
begin
    if pg_catalog.has_function_privilege('anon','public.werdy_account_delete()','execute') then raise exception 'anon RPC grant'; end if;
end $$;
reset role;
rollback;
