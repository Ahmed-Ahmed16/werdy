-- Run after the migration, as the database owner, on a test database.
-- All test users and receipts roll back. No Supabase project is contacted here.
begin;
insert into auth.users(id) values ('00000000-0000-4000-8000-000000000023'),
    ('00000000-0000-4000-8000-000000000024');

-- Rust also reads this fixture to check that SQL and serde accept the same wire
-- format. It covers all change kinds and nested enum layouts.
do $$
declare
    operations jsonb := $wire$[
      {"install":"wire","operation":1,"change":{"Deed":{"id":{"install":"wire","sequence":1},"row":{"created_on":20754,"archived_from":20758,"past_archives":[[20754,20755]],"name":{"text":"Prayer"},"starter_key":null}}}},
      {"install":"wire","operation":2,"change":{"Rule":{"id":{"install":"wire","sequence":1},"from_day":20754,"rule":{"schedule":"every_day","target":null,"category":"prayer","slot":"fajr"}}}},
      {"install":"wire","operation":3,"change":{"Rule":{"id":{"install":"wire","sequence":1},"from_day":20755,"rule":{"schedule":{"weekdays":["monday","friday"]},"target":{"amount":5,"unit":"pages"},"category":"quran","slot":"night"}}}},
      {"install":"wire","operation":4,"change":{"Rule":{"id":{"install":"wire","sequence":1},"from_day":20756,"rule":{"schedule":{"hijri":{"month":9,"first_day":1,"last_day":30}},"target":null,"category":"fasting","slot":"any_time"}}}},
      {"install":"wire","operation":5,"change":{"Mark":{"id":{"install":"wire","sequence":1},"day":20754,"entry":{"mark":"done","entered_on":20754}}}},
      {"install":"wire","operation":6,"change":{"Mark":{"id":{"install":"wire","sequence":1},"day":20755,"entry":{"mark":{"partial":{"numerator":1,"denominator":2}},"entered_on":20755}}}},
      {"install":"wire","operation":7,"change":{"Mark":{"id":{"install":"wire","sequence":1},"day":20756,"entry":{"mark":{"count":3},"entered_on":20756}}}},
      {"install":"wire","operation":8,"change":{"Mark":{"id":{"install":"wire","sequence":1},"day":20757,"entry":{"mark":"missed","entered_on":20757}}}},
      {"install":"wire","operation":9,"change":{"Mark":{"id":{"install":"wire","sequence":1},"day":20758,"entry":null}}},
      {"install":"wire","operation":10,"change":{"Note":{"day":20754,"note":{"text":"ill","credit":"half","entered_on":20754}}}},
      {"install":"wire","operation":11,"change":{"Note":{"day":20755,"note":{"text":"","credit":"full","entered_on":20755}}}},
      {"install":"wire","operation":12,"change":{"Note":{"day":20756,"note":{"text":"travel","credit":"none","entered_on":20756}}}},
      {"install":"wire","operation":13,"change":{"Note":{"day":20757,"note":null}}},
      {"install":"wire","operation":14,"change":{"Setting":{"Language":"ar"}}},
      {"install":"wire","operation":15,"change":{"Setting":{"Theme":"dark"}}},
      {"install":"wire","operation":16,"change":{"Setting":{"Digits":"arabic_indic"}}},
      {"install":"wire","operation":17,"change":{"Setting":{"Calendar":{"first_offset":1,"later_offsets":[[20754,0],[20755,-1]]}}}},
      {"install":"wire","operation":18,"change":{"ChosenSet":{"key":"prayers","chosen":true}}},
      {"install":"wire","operation":19,"change":{"Deed":{"id":{"install":"wire","sequence":2},"row":null}}},
      {"install":"wire","operation":20,"change":{"Deed":{"id":{"install":"wire","sequence":3},"row":{"created_on":20754,"archived_from":null,"past_archives":[],"name":{"key":"starter-fajr"},"starter_key":"fajr"}}}}
    ]$wire$;
    operation_json jsonb;
begin
    for operation_json in select pg_catalog.jsonb_array_elements(operations) loop
        assert werdy_private.sync_valid(operation_json,'pending'), 'wire fixture rejected';
    end loop;
end $$;

do $$
begin
    assert not pg_catalog.has_table_privilege('anon','public.werdy_sync_changes','select');
    assert not pg_catalog.has_table_privilege('authenticated','public.werdy_sync_changes','insert,update,delete,truncate');
    assert not pg_catalog.has_table_privilege('authenticated','public.werdy_sync_heads','insert,update,delete,truncate');
    assert not pg_catalog.has_function_privilege('anon','public.werdy_sync_push(jsonb)','execute');
    assert not pg_catalog.has_function_privilege('anon','public.werdy_sync_pull(numeric)','execute');
    assert not pg_catalog.has_function_privilege('anon','public.werdy_sync_server_time()','execute');
    assert not pg_catalog.has_function_privilege('authenticated','werdy_private.sync_valid(jsonb,text)','execute');
end $$;

set local request.jwt.claim.sub = '00000000-0000-4000-8000-000000000023';
set local role authenticated;
do $$
declare
    first jsonb;
    retry jsonb;
    pulled jsonb;
    batch jsonb := '[
        {"install":"phone","operation":1,"change":{"ChosenSet":{"key":"prayers","chosen":true}}},
        {"install":"phone","operation":18446744073709551615,"change":{"Note":{"day":20754,"note":{"text":"ill","credit":"half","entered_on":20754}}}}
    ]';
    rejected boolean;
    bad jsonb;
begin
    first := public.werdy_sync_push(batch);
    assert first->>'cursor' = '2';
    assert first#>>'{changes,0,stamp,sequence}' = '1';
    assert first#>>'{changes,1,stamp,sequence}' = '2';
    assert first#>>'{changes,1,pending,operation}' = '18446744073709551615';
    assert (first#>>'{changes,1,stamp,received_at}')::bigint >= (first#>>'{changes,0,stamp,received_at}')::bigint;
    retry := public.werdy_sync_push(pg_catalog.jsonb_build_array(batch->1, batch->0));
    assert retry->>'cursor' = '2';
    assert retry#>'{changes,0}' = first#>'{changes,1}';
    assert retry#>'{changes,1}' = first#>'{changes,0}';
    pulled := public.werdy_sync_pull(0);
    assert pulled = first;
    assert public.werdy_sync_pull(2) = '{"changes":[],"cursor":2}'::jsonb;
    assert public.werdy_sync_server_time() > 0;
    assert (select pg_catalog.count(*) from public.werdy_sync_changes) = 2;

    -- A changed retry after a new operation rolls back that new operation too.
    rejected := false;
    begin
        perform public.werdy_sync_push('[
            {"install":"phone","operation":3,"change":{"Note":{"day":20754,"note":null}}},
            {"install":"phone","operation":1,"change":{"ChosenSet":{"key":"prayers","chosen":false}}}
        ]');
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected;
    assert public.werdy_sync_pull(0) = first;
    retry := public.werdy_sync_push('[{"install":"phone","operation":3,"change":{"Note":{"day":20754,"note":null}}}]');
    assert retry->>'cursor' = '3';

    for bad in select pg_catalog.jsonb_array_elements('[
        null, {}, [null],
        [{"install":"phone","operation":0,"change":{"Setting":{"Language":"en"}}}],
        [{"install":"phone","operation":4,"change":{"Setting":{"Location":{"latitude":30}}}}],
        [{"install":"phone","operation":4,"change":{"Setting":{"PrayerMethod":"umm_al_qura"}}}],
        [{"install":"phone","operation":4,"change":{"Note":{"day":20754,"note":{"text":"","credit":"none","entered_on":20754}}}}],
        [{"install":"phone","operation":4,"change":{"Mark":{"id":{"install":"phone","sequence":1},"day":20754,"entry":{"mark":{"partial":{"numerator":1,"denominator":0}},"entered_on":20754}}}}],
        [{"install":"phone","operation":4,"owner":"spoof","change":{"Setting":{"Language":"en"}}}],
        [{"install":"phone","operation":4,"change":{"Setting":{"Calendar":{"first_offset":1,"later_offsets":[[2,1],[1,0]]}}}}]
    ]') loop
        rejected := false;
        begin perform public.werdy_sync_push(bad);
        exception when invalid_parameter_value then rejected := true;
        end;
        assert rejected, 'malformed operation accepted';
    end loop;
    rejected := false;
    begin perform public.werdy_sync_pull(4);
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected;
    rejected := false;
    begin perform public.werdy_sync_push(pg_catalog.jsonb_build_array(pg_catalog.jsonb_build_object(
        'install','phone','operation',4,'change',pg_catalog.jsonb_build_object('Note',pg_catalog.jsonb_build_object(
            'day',20754,'note',pg_catalog.jsonb_build_object('text',pg_catalog.repeat('x',8193),'credit','half','entered_on',20754))))));
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected, 'oversize note accepted';

    rejected := false;
    begin
        perform public.werdy_sync_push((select pg_catalog.jsonb_agg(batch->0)
            from pg_catalog.generate_series(1,1025)));
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected, 'oversize operation count accepted';
    rejected := false;
    begin
        perform public.werdy_sync_push((select pg_catalog.jsonb_agg(
            pg_catalog.jsonb_build_object('install','phone','operation',5,'change',
                pg_catalog.jsonb_build_object('Note',pg_catalog.jsonb_build_object('day',20754,
                    'note',pg_catalog.jsonb_build_object('text',pg_catalog.repeat('x',2048),
                        'credit','half','entered_on',20754))))) from pg_catalog.generate_series(1,1024)));
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected, 'oversize batch bytes accepted';
    rejected := false;
    begin
        perform public.werdy_sync_push(pg_catalog.jsonb_build_array(
            pg_catalog.jsonb_build_object('install','phone','operation',5,'change',
                pg_catalog.jsonb_build_object('Setting',pg_catalog.jsonb_build_object('Calendar',
                    pg_catalog.jsonb_build_object('first_offset',0,'later_offsets',
                        (select pg_catalog.jsonb_agg(pg_catalog.jsonb_build_array(day_number,0))
                            from pg_catalog.generate_series(1,4000) as days(day_number))))))));
    exception when invalid_parameter_value then rejected := true;
    end;
    assert rejected, 'oversize operation bytes accepted';
    assert public.werdy_sync_pull(0)->>'cursor' = '3';
end $$;
reset role;

-- Advancing the saved server time simulates a server wall clock moving back.
update public.werdy_sync_heads set last_received_at = 9000000000000
    where owner = '00000000-0000-4000-8000-000000000023';
set local role authenticated;
do $$
declare reply jsonb;
begin
    reply := public.werdy_sync_push('[{"install":"phone","operation":4,"change":{"Setting":{"Language":"en"}}}]');
    assert reply#>>'{changes,0,stamp,received_at}' = '9000000000000';
    assert reply->>'cursor' = '4';
end $$;
reset role;

set local request.jwt.claim.sub = '00000000-0000-4000-8000-000000000024';
set local role authenticated;
do $$
declare reply jsonb;
begin
    assert (select pg_catalog.count(*) from public.werdy_sync_changes) = 0;
    assert (select pg_catalog.count(*) from public.werdy_sync_heads) = 0;
    assert public.werdy_sync_pull(0) = '{"changes":[],"cursor":0}'::jsonb;
    reply := public.werdy_sync_push('[{"install":"phone","operation":1,"change":{"Setting":{"Language":"ar"}}}]');
    assert reply->>'cursor' = '1';
    assert (select pg_catalog.count(*) from public.werdy_sync_changes) = 1;
end $$;
reset role;

set local request.jwt.claim.sub = '';
set local role authenticated;
do $$
declare rejected boolean;
begin
    rejected := false;
    begin perform public.werdy_sync_push('[]'); exception when insufficient_privilege then rejected := true; end;
    assert rejected;
    rejected := false;
    begin perform public.werdy_sync_pull(0); exception when insufficient_privilege then rejected := true; end;
    assert rejected;
    rejected := false;
    begin perform public.werdy_sync_server_time(); exception when insufficient_privilege then rejected := true; end;
    assert rejected;
end $$;
reset role;
rollback;
