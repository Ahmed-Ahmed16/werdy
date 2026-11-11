begin;

alter table public.werdy_push_subscriptions
    drop constraint if exists werdy_push_subscriptions_endpoint_host_check;

alter table public.werdy_push_subscriptions
    add constraint werdy_push_subscriptions_endpoint_host_check
    check (endpoint ~* '^https://(fcm[.]googleapis[.]com|updates[.]push[.]services[.]mozilla[.]com|([a-z0-9-]+[.])+push[.]apple[.]com|([a-z0-9-]+[.])+notify[.]windows[.]com)/')
    not valid;

delete from public.werdy_push_subscriptions
    where endpoint !~* '^https://(fcm[.]googleapis[.]com|updates[.]push[.]services[.]mozilla[.]com|([a-z0-9-]+[.])+push[.]apple[.]com|([a-z0-9-]+[.])+notify[.]windows[.]com)/';

alter table public.werdy_push_subscriptions
    validate constraint werdy_push_subscriptions_endpoint_host_check;

commit;
