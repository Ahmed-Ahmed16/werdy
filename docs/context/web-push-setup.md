# Web push setup

Migrations and Edge Functions deploy to the live project when changes merge to `main`. Push setup runs separately so the private key never needs to pass through a person. Do not put private keys in this repo. Phone delivery and the Deno runtime still need a real test.

## Owner setup through GitHub Actions

1. Add the `SUPABASE_ACCESS_TOKEN` repository secret once. Create a token in the Supabase dashboard under Account, Access Tokens. Add it in GitHub repo Settings, Secrets and variables, Actions. `SUPABASE_DB_URL` is already configured.
2. After the Supabase deploy workflow succeeds on `main`, open Actions, Push setup, Run workflow. Select `main`. Enter a contact URI in `subject`, such as `mailto:you@example.com`. To keep the default `mailto:`, first set the repository variable `PUSH_CONTACT_EMAIL` to your contact email.
3. Copy the public key from the run summary into `VAPID_PUBLIC_KEY` in `crates/werdy-app/src/account/config.rs`, then merge that change. Never rotate existing VAPID keys: new keys break existing subscriptions. The workflow keeps existing keys and saves a public copy in Vault for later summaries. For keys created before this workflow, use the public key from the original setup records.
4. Check delivery on phones as described below. Deployment keeps `push-send` without JWT verification because it checks its own bearer secret.

The setup workflow needs the `20261114000000_push_cron.sql` migration and Vault enabled. Reruns update the project URL and replace the named Cron job. They change the Vault cron secret only when this run sets a new function cron secret. Existing function secrets stay unchanged. The public key marker is saved last, so a failed first setup can retry. For existing keys with a missing Vault cron secret, use the manual fallback to restore the matching value. Do not delete existing VAPID keys to repair a setup.

## Manual fallback

Use these steps if Actions is unavailable. Keep private values on a trusted computer or in the Supabase dashboard.

### 1. Generate the VAPID keys

Owner step. After approval, run `npx --yes web-push@3.6.7 generate-vapid-keys --json` on a trusted computer. Keep the private key secret. Keep the public key for the app. Use one key pair for all installs so existing subscriptions keep working.

### 2. Set function secrets

Owner step. After approval, use the Supabase dashboard Edge Function secrets page. Set `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`, and `VAPID_SUBJECT`. The subject is a contact URI such as `mailto:you@example.com`. Set `PUSH_CRON_SECRET` to a long random value. Put the same value in Vault in step 4. Supabase supplies `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` to Edge Functions. Never put the service role key in the web app.

### 3. Put the public key in the app and deploy

Owner step. After approval, set the existing `VAPID_PUBLIC_KEY` constant to the public key in `crates/werdy-app/src/account/config.rs`. It is public by design. Check it matches the private key before people subscribe.

Owner step. After approval, run `supabase functions deploy push-send --no-verify-jwt` against the linked project. The function checks `Authorization: Bearer <PUSH_CRON_SECRET>` itself. Test it in a test project before use. The Web Push library is `npm:web-push@3.6.7`. It generates VAPID headers and `aes128gcm` encrypted bodies. Its Deno runtime behavior is `___` because Deno and network access were unavailable during this ticket. Confirm the import and a real test push in the Edge Function runtime before scheduling Cron.

### 4. Store the Cron secret and schedule the call

Owner step. After approval, run this SQL in the Supabase SQL Editor as the database owner. Replace `<project ref>` and `<same random cron secret>` first. The secret must match `PUSH_CRON_SECRET` in step 2. The URL is kept in Vault too. For an existing Vault name, use `vault.update_secret(id, new_secret)` with its ID from `vault.secrets` instead of `vault.create_secret`. Keep the cron secret equal to the function secret. Calling `cron.schedule` with the same name replaces the job.

```sql
select vault.create_secret(
  'https://<project ref>.supabase.co',
  'werdy_push_project_url'
);

select vault.create_secret(
  '<same random cron secret>',
  'werdy_push_cron_secret'
);

select cron.schedule(
  'werdy-push-each-minute',
  '* * * * *',
  $$
  select net.http_post(
    url := (
      select decrypted_secret from vault.decrypted_secrets
      where name = 'werdy_push_project_url'
    ) || '/functions/v1/push-send',
    headers := jsonb_build_object(
      'Content-Type', 'application/json',
      'Authorization', 'Bearer ' || (
        select decrypted_secret from vault.decrypted_secrets
        where name = 'werdy_push_cron_secret'
      )
    ),
    body := '{}'::jsonb,
    timeout_milliseconds := 5000
  ) as request_id;
  $$
);
```

Supabase must have `pg_cron`, `pg_net`, and Vault enabled. Check their state in the dashboard before running the SQL. The Cron request calls the function each minute. Each claim calls `werdy_push_claim_due` with `{"max_rows":20}` and returns a JSON array. The sender handles up to 10 push requests at once. Each push request has a 10 second timeout. It stops claiming when a batch is empty or 100 seconds have passed since the run began. Unclaimed due rows wait for a later run, subject to the migration's 15 minute expiry. The claim RPC removes claimed rows before sending. Failed sends do not retry. A 404 or 410 also removes the subscription.

Push endpoints must use HTTPS on the default port at `fcm.googleapis.com`, `updates.push.services.mozilla.com`, a subdomain of `push.apple.com`, or a subdomain of `notify.windows.com`. The migration removes stored subscriptions outside this list, the database rejects new ones, and the sender drops any remaining invalid subscription before encryption or sending.

## 5. Check on phones

Owner step. After approval and deployment, turn reminders on and close werdy. Check one due reminder on Android Chrome. On iPhone, add werdy to the home screen first, grant notification permission from a tap, then close it and check a due reminder. Tap each notification and check that today's sheet opens. Record device, OS version, browser, time, and result in the PR. Status now: `___` for both phones.

## Local test

Run `node --test supabase/functions/push-send/core.test.ts` from the repo root. This checks payload, delivery headers, endpoint rules, reply handling, send concurrency, and the claim time budget without Deno or network access.
The `package.json` in that folder exists only for Node tests and declares no dependencies.
