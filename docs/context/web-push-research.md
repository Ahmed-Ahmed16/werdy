# Web push from a GitHub Pages app with Supabase

Research for issue #118, part of map #117. Written 2026-11-11. Every claim cites the primary source it came from. Where a source did not say something, this file says so instead of guessing.

## Question

What does it take to send a push to a closed werdy web app at a set time (prayer plus a delay), on Android and iPhone, served from GitHub Pages, sent from Supabase on the free tier, without storing users' locations on the server?

## Short answer

It is possible with standards only and no paid service. The phone subscribes with the Push API, the subscription (endpoint plus two keys) goes to Supabase, and a Supabase Cron job calls an Edge Function every minute that sends the due pushes with VAPID and `aes128gcm` encryption. The browser's push service (FCM for Chrome, Apple's service for Safari) wakes the service worker even when werdy is closed, and the service worker shows the notification.

The limits that matter: on iPhone it works only for a werdy added to the home screen, on iOS 16.4 or later, after a tap that asks for permission, and every push must show a notification at once. A free Supabase project pauses after a week without activity. Nothing on the web can schedule a local notification, so the server must send each reminder at the right minute.

## How werdy is built and served today

* `.github/workflows/ci.yml`, job `pages`: `dx build --platform web --release --base-path werdy`, then `scripts/prepare-pages.ps1`, then the GitHub Pages deploy. The site lives under `https://<owner>.github.io/werdy/`.
* `scripts/prepare-pages.ps1` fills `scripts/service-worker.template.js` with a version hash and a precache list and writes `service-worker.js` into the build root.
* `crates/werdy-app/index.html` registers it with `navigator.serviceWorker.register(worker.href, { scope: "/werdy/" })`.
* `crates/werdy-app/public/manifest.webmanifest` has `"display": "standalone"`, `"start_url": "/werdy/"` and `"scope": "/werdy/"`. It has no `id` member.

So the service worker already exists, is plain JavaScript (not Rust), lives at `/werdy/service-worker.js`, and controls the whole app. Push needs only two new listeners in the template (`push` and `notificationclick`, plus `pushsubscriptionchange`), not a new file or build step. The manifest already meets Apple's rule for a home screen web app. Adding a fixed `id` (for example `"/werdy/"`) is worth doing, since iOS uses it to identify the installed app and sync Focus settings ([WebKit, Web Push for Web Apps on iOS and iPadOS](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/)).

## 1. The Web Push standard

Three IETF RFCs and one W3C spec make up Web Push.

* **Push API** ([W3C Push API](https://www.w3.org/TR/push-api/), Working Draft dated 2026-11-09). `PushManager.subscribe()` takes `userVisibleOnly` and `applicationServerKey`. The key is a P-256 public key, 65 octets, base64url when given as a string (section 7.3). `subscribe()` rejects if the registration has no active service worker (`InvalidStateError`) or if the scope is not `https` (section 7.1). Push messages go to the active service worker, and the browser starts it if it is not running (section 3.2), so the app does not need to be open.
* **Delivery protocol** ([RFC 8030](https://www.rfc-editor.org/rfc/rfc8030.html)). The sender POSTs to the subscription's endpoint URL.
  * `TTL` is required, in seconds. With `TTL: 0` the message is dropped if the phone is not reachable at that moment. The push service may keep a message for less time than asked and says so in the response (section 5.2).
  * `Urgency`: `very-low`, `low`, `normal` (default) or `high` (section 5.3).
  * `Topic`: up to 32 base64url characters. A new message with the same topic replaces an undelivered one for that subscription (section 5.4).
  * A push service must accept bodies up to 4096 bytes (section 7.2).
  * 404 means the subscription has expired (sections 6.1, 7.3). 410 reports delivery failure (section 6.2). 429 means slow down, with `Retry-After` (section 8.4).
* **VAPID** ([RFC 8292](https://www.rfc-editor.org/rfc/rfc8292.html)). The server holds one P-256 key pair. Each request carries `Authorization: vapid t=<JWT>, k=<public key>`. The JWT is ES256 signed and has `aud` (the push service origin), `exp` (no more than 24 hours ahead) and optionally `sub` (a `mailto:` or `https:` contact) (section 2). A subscription made with that public key only accepts messages signed by the matching private key (section 4).
* **Payload encryption** ([RFC 8291](https://www.rfc-editor.org/rfc/rfc8291.html)). Content coding `aes128gcm` (RFC 8188), ECDH against the subscription's P-256 key plus its 16 octet auth secret, one record per message. Usable plaintext is at most 3993 octets (section 4). The push service cannot read the text.

Note: [web.dev, The Web Push Protocol](https://web.dev/articles/push-notifications-web-push-protocol) still documents the older `aesgcm` coding and the `WebPush` header form. Follow the RFCs and use `aes128gcm` with `Authorization: vapid`.

What werdy stores per subscription: the endpoint URL, the `p256dh` key and the `auth` secret (the three fields of `PushSubscription.toJSON()`), plus the owner id. The VAPID private key lives only as a Supabase Edge Function secret.

## 2. A service worker for the Dioxus app on GitHub Pages

* **Where it lives.** `/werdy/service-worker.js`, built from `scripts/service-worker.template.js` by `prepare-pages.ps1`. Its scope `/werdy/` matches the manifest scope. GitHub Pages serves it over HTTPS, which the Push API requires.
* **What to add.** A `push` listener that parses the JSON payload and calls `self.registration.showNotification(...)` inside `event.waitUntil`, a `notificationclick` listener that focuses or opens `/werdy/`, and a `pushsubscriptionchange` listener (see section 6).
* **Rust side.** Subscribing happens in the page, not the worker. The Rust app can call `navigator.serviceWorker.ready`, then `pushManager.subscribe({ userVisibleOnly: true, applicationServerKey })` through `web-sys` or a small JavaScript shim, from a button's click handler.
* **Keep the precache logic.** The push listeners do not touch the fetch and cache code. A new deploy changes the version hash, which installs a new worker. The push subscription belongs to the registration, not to the worker version, so an update does not drop it ([W3C Push API](https://www.w3.org/TR/push-api/), section 3.4).

## 3. iPhone and Android limits

### iPhone and iPad

* **Version.** Web Push for home screen web apps arrived in iOS and iPadOS 16.4 ([WebKit](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/); [Apple, Sending web push notifications in web apps and browsers](https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers)).
* **Home screen only.** The site must be added to the home screen through the Share menu, and it becomes a web app only if its manifest `display` is `standalone` or `fullscreen` ([WebKit](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/)). werdy's manifest qualifies. A werdy opened in a Safari tab on iPhone cannot get push.
* **Permission prompt.** Ask only from a user gesture, such as a tap on a button, and call subscribe straight from that handler ([Apple](https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers)). Asking on page load will not work.
* **Every push must show a notification.** "Safari doesn't support invisible push notifications", and if the service worker does not present one immediately, Safari revokes the permission ([Apple](https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers)). So no silent "refresh your schedule" pushes on iPhone.
* **Declarative Web Push** (iOS and iPadOS 18.4 and later). A payload with `"web_push": 8030` and a `notification` object (`title` and a `navigate` URL required) is shown by the system even if the service worker fails or does not run in time, with no penalty ([WebKit, Meet Declarative Web Push](https://webkit.org/blog/16535/meet-declarative-web-push/); [WebKit Features in Safari 18.4](https://webkit.org/blog/16574/webkit-features-in-safari-18-4/)). The format is backward compatible: older browsers hand the same JSON to the service worker. werdy should send this format, so the revocation risk on iPhone goes away on current iOS.
* **Server side.** Apple's endpoints are on `*.push.apple.com`. Apple stores an undeliverable message for at most 30 days and limits how many it stores while the device is offline. Payloads are limited to 4 KB. Do not refresh the VAPID JWT more often than once an hour. 410 means "The device token has expired". 429 means too many requests to the same device ([Apple](https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers)).
* **Badges.** `navigator.setAppBadge` works while handling a push ([WebKit](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/)). Optional.

### Android Chrome

* Chrome requires `userVisibleOnly: true`; omitting it is an error ([web.dev, Subscribing a user](https://web.dev/articles/push-notifications-subscribing-a-user)).
* Push works whether the app is open, in the background or not loaded; the browser starts the service worker ([MDN, Push API](https://developer.mozilla.org/en-US/docs/Web/API/Push_API)). Installing to the home screen is not required on Android, but it helps.
* A blocked prompt can only be undone in settings, so ask at a moment that explains why ([web.dev](https://web.dev/articles/push-notifications-subscribing-a-user)).
* `Urgency: high` asks the push service to deliver now rather than wait for a power friendly moment ([RFC 8030](https://www.rfc-editor.org/rfc/rfc8030.html) section 5.3; Apple says `high` to "attempt to deliver the notification immediately"). For a reminder at a set minute, use `high`.

### What the web cannot do

* **No local scheduled notifications.** Chrome's Notification Triggers (`TimestampTrigger`) was ended: "The development of Notification Triggers API ... has ended" ([Chrome for Developers](https://developer.chrome.com/docs/web-platform/notification-triggers)). So a closed web app cannot set its own alarm; a server must push at the right time.
* **No dependable background wakeups.** Periodic Background Sync needs an installed app, a positive engagement score and a known network, and the browser picks the timing ([Chrome for Developers](https://developer.chrome.com/docs/capabilities/periodic-background-sync)). The Chrome page does not list other browsers, and none of the Apple or WebKit pages read mention it, so iPhone support is `___` and should be assumed absent. Either way it cannot be relied on to refresh reminder times.

## 4. Sending from Supabase

* **Scheduler.** Supabase Cron is `pg_cron`. Jobs can run from every second to once a year. Supabase recommends no more than 8 concurrent jobs and no job longer than 10 minutes ([Supabase Cron](https://supabase.com/docs/guides/cron)).
* **Calling the sender.** The documented pattern is `cron.schedule('...', '* * * * *', $$ select net.http_post(...) $$)` with the project URL and a secret key read from Vault, and the function set to accept that key ([Scheduling Edge Functions](https://supabase.com/docs/guides/functions/schedule-functions)). `pg_net` is asynchronous, defaults to a 2000 ms timeout, keeps responses 6 hours, and is tuned for up to 200 requests per second ([pg_net](https://supabase.com/docs/guides/database/extensions/pg_net)). Caution from the same page: while queued, the decrypted key sits in `net.http_request_queue`, readable by any role that can log in.
* **The sender.** One Edge Function run each minute: select reminders due in this minute (and any missed in the last few minutes), sign one VAPID JWT per push service origin (reused for up to an hour, per Apple's rule), encrypt each payload, POST to each endpoint, delete the subscription on 404 or 410. Edge Functions can import npm and JSR packages ([Dependencies](https://supabase.com/docs/guides/functions/dependencies)), so an existing Web Push library can do the crypto. Whether a given library runs on the Deno runtime is `___` until a spike tries it.
* **Edge Function limits** ([Limits](https://supabase.com/docs/guides/functions/limits)): 256 MB memory, 150 s wall clock on the free plan, 2 s CPU per request (async I/O waiting does not count), 150 s idle timeout.
* **Free plan** ([Pricing](https://supabase.com/pricing)): 500,000 Edge Function invocations a month, 500 MB database, 50,000 monthly active users, 5 GB egress, projects "paused after 1 week of inactivity". Two free projects ([Billing](https://supabase.com/docs/guides/platform/billing-on-supabase)).
* **Budget check.** One run a minute is about 44,640 invocations in a 31 day month, under 9 percent of the free quota. At family scale (decision 0006), each minute has a handful of pushes; five prayers a day for 20 people is 100 pushes a day. Each push costs one ECDH and one AES encryption, small against the 2 s CPU budget. Many thousands of pushes in one minute would need batching across several runs; that is far beyond v1.
* **Pausing.** Whether a running `pg_cron` job counts as activity that stops the one week pause is not stated in the docs read (`___`). If the project pauses, all reminders stop silently. This needs a check before relying on the free tier.

## 5. Expired and dead subscriptions

* `PushSubscription.expirationTime` is a time in ms or `null`. The browser should refresh before then ([W3C Push API](https://www.w3.org/TR/push-api/), section 8).
* `pushsubscriptionchange` fires in the service worker when a subscription is refreshed, revoked or lost. On a refresh it carries both the old and new subscription; when the subscription is gone, `newSubscription` is `null` (section 10.4). The worker should send the new one to Supabase, replacing the old row. Doing that from the worker needs the user's Supabase session, which today lives in `localStorage`, which a service worker cannot read. Either keep a copy of the refresh token in IndexedDB or let the next app open repair the subscription.
* On send: 404 (expired, RFC 8030 section 7.3) and 410 (gone; Apple: "The device token has expired") mean delete the row ([web.dev, The Web Push Protocol](https://web.dev/articles/push-notifications-web-push-protocol); [Apple](https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers)). 429 means wait for `Retry-After`. 413 means the payload is too big.
* Every time the app opens it should compare `pushManager.getSubscription()` with what the server has, and upload it again if they differ. This catches losses the worker missed.

## 6. The favoured design: the phone uploads precomputed UTC times

The phone runs the planner in `crates/werdy-reminders` (decision 0038), gets the next N reminders as UTC instants with a stable id and a short label, and uploads only those. The server sends at those instants. No coordinates reach the server.

What fits well:

* The planner already makes stable ids, so a reupload replaces rather than duplicates. The id, shortened to 32 base64url characters, can also be the `Topic`, so two pushes for the same reminder collapse into one on an offline phone ([RFC 8030](https://www.rfc-editor.org/rfc/rfc8030.html) section 5.4).
* The server stays simple: a table of `(owner, subscription, send_at, reminder_id, payload)` and a "due now" query. No prayer time math on the server.
* The planner's "is it still due and not Done" check runs on the phone at upload time. When a deed is marked Done, the app deletes that row.

What argues against it, or needs care:

1. **Times still reveal place.** A list of prayer times in UTC is a fingerprint of location. Maghrib in UTC tracks longitude, and the gap between Fajr and Maghrib over several days tracks latitude. Even with the user's delays added, the server could work out roughly which city. This is weaker than storing coordinates, but it is not "no location". Upload a short window (a few days), and do not keep sent rows, to limit what the server ever holds. This inference is my own reasoning, not a sourced fact; it should be stated plainly in the privacy copy.
2. **The times run out.** If the phone does not open werdy, nobody refreshes the list. The web has no reliable background wakeup (section 3), and iPhone forbids silent pushes. One partial fix: each reminder push wakes the service worker, which must show a notification anyway, and could also top up the schedule. That needs the planner and the user's session inside the worker, which today hold neither (the planner is Rust in the page, the session in `localStorage`). The map's open item "what the server sends when the uploaded reminder times run out" stays open; a "please open werdy" push on the last stored day is the simplest answer.
3. **Done marks change after upload.** The server cannot know a deed was marked Done on another device unless sync deletes the row. Rule 0038 says one slot reminder covers only due, not Done deeds. Either the app deletes rows when marks change (needs a network round trip at mark time), or the push body stays generic ("Dhuhr deeds") and the service worker checks local data before showing. On iPhone the worker cannot skip showing, so a generic body is safer.
4. **Moving and time zones.** Travel changes prayer times. The phone must replan and reupload when its location or the day changes; until then, reminders follow the old place.
5. **Free tier pause.** All reminders depend on one free project staying awake (section 4).

None of these block the design. Point 1 is the one to settle with the owner, since the goal was "no location on the server".

## Sources

* W3C, Push API (Working Draft, 2026-11-09): https://www.w3.org/TR/push-api/
* IETF RFC 8030, Generic Event Delivery Using HTTP Push: https://www.rfc-editor.org/rfc/rfc8030.html
* IETF RFC 8291, Message Encryption for Web Push: https://www.rfc-editor.org/rfc/rfc8291.html
* IETF RFC 8292, VAPID for Web Push: https://www.rfc-editor.org/rfc/rfc8292.html
* Apple, Sending web push notifications in web apps and browsers: https://developer.apple.com/documentation/usernotifications/sending-web-push-notifications-in-web-apps-and-browsers
* WebKit, Web Push for Web Apps on iOS and iPadOS: https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/
* WebKit, Meet Declarative Web Push: https://webkit.org/blog/16535/meet-declarative-web-push/
* WebKit, WebKit Features in Safari 18.4: https://webkit.org/blog/16574/webkit-features-in-safari-18-4/
* MDN, Push API: https://developer.mozilla.org/en-US/docs/Web/API/Push_API
* web.dev, Subscribing a user: https://web.dev/articles/push-notifications-subscribing-a-user
* web.dev, The Web Push Protocol: https://web.dev/articles/push-notifications-web-push-protocol
* Chrome for Developers, Notification Triggers: https://developer.chrome.com/docs/web-platform/notification-triggers
* Chrome for Developers, Periodic Background Sync: https://developer.chrome.com/docs/capabilities/periodic-background-sync
* Supabase, Cron: https://supabase.com/docs/guides/cron
* Supabase, Scheduling Edge Functions: https://supabase.com/docs/guides/functions/schedule-functions
* Supabase, pg_net: https://supabase.com/docs/guides/database/extensions/pg_net
* Supabase, Edge Function limits: https://supabase.com/docs/guides/functions/limits
* Supabase, Edge Function dependencies: https://supabase.com/docs/guides/functions/dependencies
* Supabase, Pricing: https://supabase.com/pricing
* Supabase, Billing: https://supabase.com/docs/guides/platform/billing-on-supabase
