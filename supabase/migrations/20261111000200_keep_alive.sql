-- Apply as the database owner before enabling the keep alive workflow.
begin;

create or replace function public.werdy_ping()
returns boolean language sql stable set search_path = '' as $$
    select true from pg_catalog.pg_class limit 1
$$;

revoke all on function public.werdy_ping() from public;
grant execute on function public.werdy_ping() to anon, authenticated;
notify pgrst, 'reload schema';
commit;
