-- Push setup needs Cron to schedule calls and pg_net to send them.
create extension if not exists pg_cron with schema pg_catalog;
create extension if not exists pg_net with schema extensions;
