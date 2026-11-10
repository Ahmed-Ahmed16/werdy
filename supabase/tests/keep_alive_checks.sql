-- Run as database owner after the keep alive migration. Everything rolls back.
begin;
set local role anon;
do $$
begin
    if public.werdy_ping() is distinct from true then
        raise exception 'keep alive ping failed';
    end if;
end $$;
rollback;
