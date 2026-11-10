-- Run as database owner after all four migrations. Everything rolls back.
begin;
set local request.jwt.claim.sub = '';
create temporary table push_test(first_user uuid, second_user uuid, first_id uuid, deleted_id uuid);
insert into push_test values(pg_catalog.gen_random_uuid(), pg_catalog.gen_random_uuid(), null, null);
grant select, update on push_test to authenticated;
insert into auth.users(id,email) select first_user,'first@push.test' from push_test
    union all select second_user,'second@push.test' from push_test;

select pg_catalog.set_config('request.jwt.claims',
    pg_catalog.jsonb_build_object('sub',first_user,'role','authenticated')::text,true) from push_test;
set local role authenticated;
do $$
declare
    subscription jsonb := '{"endpoint":"https://push.example.test/first","p256dh":"key_1","auth":"auth_1"}';
    now_ms bigint := pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint;
    item jsonb;
    rejected boolean;
begin
    item := pg_catalog.jsonb_build_object('id','first','send_at_ms',now_ms + 60000,
        'title','Prayer','body','Time to pray','topic','prayer_1');
    perform public.werdy_push_replace(subscription, pg_catalog.jsonb_build_array(item));
    if (select pg_catalog.count(*) from public.werdy_push_subscriptions) <> 1
        or (select pg_catalog.count(*) from public.werdy_push_reminders) <> 1 then
        raise exception 'owner cannot see push rows';
    end if;
    perform public.werdy_push_replace(pg_catalog.jsonb_build_object(
        'endpoint','https://push.example.test/boundary',
        'p256dh',pg_catalog.repeat('A',256),'auth','auth_1'), '[]');
    rejected := false;
    begin perform public.werdy_push_replace(pg_catalog.jsonb_build_object(
        'endpoint','https://push.example.test/boundary',
        'p256dh',pg_catalog.repeat('A',257),'auth','auth_1'), '[]');
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected or not exists(select 1 from public.werdy_push_subscriptions
        where endpoint = 'https://push.example.test/boundary' and pg_catalog.char_length(p256dh) = 256) then
        raise exception 'push key length boundary failed';
    end if;
    perform public.werdy_push_forget('https://push.example.test/boundary');
    update push_test set first_id = (select id from public.werdy_push_subscriptions);
    rejected := false;
    begin perform public.werdy_push_replace(subscription || '{"location":"hidden"}'::jsonb,
        pg_catalog.jsonb_build_array(item));
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'extra subscription key accepted'; end if;
    rejected := false;
    begin perform public.werdy_push_replace(subscription,
        pg_catalog.jsonb_build_array(item || '{"location":"hidden"}'::jsonb));
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'extra reminder key accepted'; end if;
    rejected := false;
    begin perform public.werdy_push_replace(subscription,
        (select pg_catalog.jsonb_agg(item) from pg_catalog.generate_series(1,51)));
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception '51 reminders accepted'; end if;
    rejected := false;
    begin perform public.werdy_push_replace(subscription, pg_catalog.jsonb_build_array(
        pg_catalog.jsonb_set(item,'{send_at_ms}',pg_catalog.to_jsonb(now_ms + 777600000))));
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'nine day reminder accepted'; end if;
    rejected := false;
    begin perform public.werdy_push_replace(subscription, pg_catalog.jsonb_build_array(
        pg_catalog.jsonb_set(item,'{send_at_ms}',pg_catalog.to_jsonb(now_ms - 960000))));
        exception when invalid_parameter_value then rejected := true; end;
    if not rejected then raise exception 'late reminder accepted'; end if;
    if (select pg_catalog.count(*) from public.werdy_push_reminders) <> 1 then
        raise exception 'rejected replacement changed rows';
    end if;
    perform public.werdy_push_replace(subscription, pg_catalog.jsonb_build_array(
        pg_catalog.jsonb_set(item,'{id}','"new"'::jsonb)));
    if (select pg_catalog.count(*) from public.werdy_push_reminders) <> 1
        or not exists(select 1 from public.werdy_push_reminders where reminder = 'new') then
        raise exception 'replacement kept old reminder';
    end if;
    rejected := false;
    begin perform public.werdy_push_claim_due();
        exception when insufficient_privilege then rejected := true; end;
    if not rejected then raise exception 'authenticated claim allowed'; end if;
end $$;
reset role;

select pg_catalog.set_config('request.jwt.claims',
    pg_catalog.jsonb_build_object('sub',second_user,'role','authenticated')::text,true) from push_test;
set local role authenticated;
do $$
begin
    if exists(select 1 from public.werdy_push_subscriptions)
        or exists(select 1 from public.werdy_push_reminders) then
        raise exception 'other user can read push rows';
    end if;
    perform public.werdy_push_forget('https://push.example.test/first');
end $$;
reset role;
do $$
begin
    if not exists(select 1 from public.werdy_push_subscriptions s
        join push_test t on t.first_id = s.id where s.owner = t.first_user) then
        raise exception 'other user forgot subscription';
    end if;
end $$;

-- Direct setup reaches the claim boundary without waiting for the clock.
insert into public.werdy_push_reminders(subscription,reminder,send_at,title,body,topic)
    select first_id,'due',pg_catalog.clock_timestamp() - interval '1 minute','Due','Send','due_1'
    from push_test;
insert into public.werdy_push_reminders(subscription,reminder,send_at,title,body,topic)
    select first_id,'expired',pg_catalog.clock_timestamp() - interval '16 minutes','Old','Skip','old_1'
    from push_test;
set local role service_role;
do $$
declare claimed record; count_claimed integer := 0;
begin
    for claimed in select * from public.werdy_push_claim_due() loop
        count_claimed := count_claimed + 1;
        if claimed.reminder <> 'due' or claimed.endpoint <> 'https://push.example.test/first'
            or claimed.p256dh <> 'key_1' or claimed.auth <> 'auth_1' then
            raise exception 'claim returned wrong row';
        end if;
    end loop;
    if count_claimed <> 1 then raise exception 'claim did not return one due row'; end if;
    if exists(select 1 from public.werdy_push_claim_due()) then
        raise exception 'claim returned row twice';
    end if;
end $$;
reset role;
do $$
begin
    if exists(select 1 from public.werdy_push_reminders where reminder in ('due','expired')) then
        raise exception 'claim kept due or expired row';
    end if;
    if not exists(select 1 from public.werdy_push_reminders where reminder = 'new') then
        raise exception 'claim removed future row';
    end if;
end $$;

select pg_catalog.set_config('request.jwt.claims',
    pg_catalog.jsonb_build_object('sub',second_user,'role','authenticated')::text,true) from push_test;
set local role authenticated;
do $$
begin
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/first","p256dh":"key_2","auth":"auth_2"}', '[]');
    if (select pg_catalog.count(*) from public.werdy_push_subscriptions) <> 1
        or (select pg_catalog.count(*) from public.werdy_push_reminders) <> 0 then
        raise exception 'endpoint takeover kept old reminders';
    end if;
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/drop","p256dh":"key_2","auth":"auth_2"}', '[]');
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/forget","p256dh":"key_2","auth":"auth_2"}', '[]');
    perform public.werdy_push_forget('https://push.example.test/forget');
    if exists(select 1 from public.werdy_push_subscriptions
        where endpoint = 'https://push.example.test/forget') then
        raise exception 'owner forget kept subscription';
    end if;
end $$;
reset role;
do $$
begin
    if not exists(select 1 from public.werdy_push_subscriptions s join push_test t on t.first_id = s.id
        where s.owner = t.second_user and s.p256dh = 'key_2') then
        raise exception 'endpoint did not move to second user';
    end if;
end $$;
set local role service_role;
do $$
begin
    perform public.werdy_push_drop_subscription(
        (select id from public.werdy_push_subscriptions where endpoint = 'https://push.example.test/drop'));
end $$;
reset role;
do $$
begin
    if exists(select 1 from public.werdy_push_subscriptions where endpoint = 'https://push.example.test/drop') then
        raise exception 'service did not drop subscription';
    end if;
end $$;

select pg_catalog.set_config('request.jwt.claims',
    pg_catalog.jsonb_build_object('sub',first_user,'role','authenticated')::text,true) from push_test;
set local role authenticated;
do $$
declare now_ms bigint := pg_catalog.floor(extract(epoch from pg_catalog.clock_timestamp()) * 1000)::bigint;
begin
    perform public.werdy_push_replace(
        '{"endpoint":"https://push.example.test/deleted","p256dh":"key_1","auth":"auth_1"}',
        pg_catalog.jsonb_build_array(pg_catalog.jsonb_build_object('id','delete','send_at_ms',now_ms + 60000,
            'title','Delete','body','Cascade','topic','delete_1')));
    update push_test set deleted_id = (select id from public.werdy_push_subscriptions
        where endpoint = 'https://push.example.test/deleted');
end $$;
reset role;
delete from auth.users u using push_test t where u.id = t.first_user;
do $$
begin
    if exists(select 1 from public.werdy_push_subscriptions s join push_test t on s.id = t.deleted_id)
        or exists(select 1 from public.werdy_push_reminders r join push_test t on r.subscription = t.deleted_id) then
        raise exception 'auth deletion kept push rows';
    end if;
    if not exists(select 1 from public.werdy_push_subscriptions s join push_test t on s.id = t.first_id
        where s.owner = t.second_user) then
        raise exception 'auth deletion removed transferred endpoint';
    end if;
    if pg_catalog.has_function_privilege('authenticated','public.werdy_push_claim_due()','execute')
        or pg_catalog.has_function_privilege('anon','public.werdy_push_claim_due()','execute')
        or pg_catalog.has_function_privilege('authenticated','public.werdy_push_drop_subscription(uuid)','execute')
        or pg_catalog.has_table_privilege('authenticated','public.werdy_push_subscriptions','insert')
        or pg_catalog.has_table_privilege('authenticated','public.werdy_push_reminders','delete') then
        raise exception 'push write or service grant leaked';
    end if;
end $$;
rollback;
