-- Apply after the sync and sharing migrations, as the database owner.
-- Google Play asks for account deletion inside the app. This removes the
-- caller's server copy, shares in both directions and the account itself.
begin;

create or replace function public.werdy_account_delete()
returns jsonb language plpgsql volatile security definer set search_path = '' as $$
declare caller uuid := werdy_private.share_caller();
begin
    -- Every table below also cascades from auth.users. Deleting explicitly
    -- keeps this function correct if a later table forgets the cascade.
    delete from werdy_private.share_codes c where c.owner = caller;
    delete from werdy_private.share_attempts a where a.watcher = caller;
    delete from public.werdy_shares s where s.owner = caller or s.watcher = caller;
    delete from public.werdy_sync_changes c where c.owner = caller;
    delete from public.werdy_sync_heads h where h.owner = caller;
    -- Identities, sessions and refresh tokens cascade from auth.users.
    delete from auth.users u where u.id = caller;
    if not found then raise exception 'account not found' using errcode = '42501'; end if;
    return pg_catalog.jsonb_build_object('ok', true);
end $$;

revoke all on function public.werdy_account_delete() from public, anon, authenticated;
grant execute on function public.werdy_account_delete() to authenticated;
notify pgrst, 'reload schema';
commit;
