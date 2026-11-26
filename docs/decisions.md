# Decisions

Newest first. Each entry: the decision, why, and what would make us revisit it.

## 0052 (2026-11-26): An older build keeps syncing past rows it cannot read
A pull reads each row on its own. When a row holds something this build does not know, such as a schedule kind added in a newer version (0047), the phone keeps that row as it came, unread, and goes on with the rest of the stream. It never pushes anything because of a row it could not read: a deed whose first rule it cannot read stays hidden on that phone, and a deed it already showed keeps its last rule it could read. After the app updates, the kept rows are read again on the next sync, without starting the pull from zero. Watching another person's sheet works the same way.

Why: before this, one unknown value in one row made the whole pull fail, so a phone on an older build stopped syncing as soon as a newer phone used "once a week" or "when it comes up". Its own data was safe, but nothing new arrived until it updated. Found in the reviews of #200 and #203; the owner chose to fix it on 2026-11-26. Revisit: if a future change needs every phone on the same build, which would need a version check instead.

## 0051 (2026-11-26): How the v2 schedules and groups meet data people already have
The owner answered these questions from the v2 reviews in chat on 2026-11-26:
* Owners who already added the advanced level do not get the two new weekly deeds (visiting graves, weekly sunan al fitra). Only a new add of the level brings them.
* Deeds already added that 0047 gives "when it comes up" (istikhara, hajah, the five makeups of missed prayers, the advanced level's fasting) stay daily. Only new adds get "when it comes up". There is no one time schedule move, unlike groups.
* A section holding only unmarked "when it comes up" deeds folds by itself (0046, 0047).
* Starter deeds that are not level deeds and sit in "any time", such as Quran reading, stay there. 0045 moves level deeds only.
* Archived level deeds also move into their groups (0045), so restoring one puts it in its group.
* The two weekly deeds, visiting graves and weekly sunan al fitra, stay in "any time"; 0045 names no group for them.

Why: each point keeps what an owner already sees unchanged unless 0045 or 0047 says otherwise, and settles a case the earlier entries left open. Revisit: if owners ask for their old deeds to take the new schedules.

## 0050 (2026-11-26): A late sign up may mark the week before
When a person finishes first run, their setup deeds start 7 days before that day (`first_markable_day` in werdy-core, `LATE_SIGN_UP_DAYS`), so they can fill in the last week. Days before that week are not due, so no statistic counts them as Missed; this is scoring rule 4 with an earlier start day, not a new rule. Deeds added after first run still start on the day they are added. Marks entered more than 2 days after their day carry the "added later" sign (0021), so most of that week shows it. The owner decided a week on 2026-11-26 (#170); a month was considered.

Why: someone who starts using werdy mid week has already prayed and read this week, and should be able to record it. Moving the start day keeps every report, strip and sheet consistent with one rule. Open: an unmarked day in that week counts as Missed, like any past due day. Revisit: if new people find a week of red they did not choose to fill, by counting only marked days in that week.

## 0049 (2026-11-26): The five prayers block is built on the congregation deed
The reports block "The five prayers" shows, for each prayer and each day, the mark of the owner's congregation deed for that prayer, from a level or the mosque prayer set. The mark already tells the whole story: done (green) is the first congregation, partial (yellow) is a later congregation, and missed (X) is a prayer prayed alone. A day with no mark at all is the real "missed or not recorded" state and must look clearly different from an X. The on time deed from the five prayers set is used only on a day no congregation deed for that prayer is due, for example when the owner has none. Each row counts done, partial, missed and unmarked days over the shown range, from pure tested functions. Which deeds belong to which prayer is the `fard_prayers` table in `data/starter-sets.toml`.

Why: the owner uses this block to see at a glance, by colour, what he missed on past days. On time and congregation as two prayer rows make no sense there, since a person prays either way, and the congregation mark carries more. This revises the block's source in 0048, which put the on time deed first. 0048 still stands for the deeds themselves: they stay two deeds, and merging the duplicate rows on the daily sheet stays parked (#183). Decided by the owner in chat on 2026-11-26 (issues #189 and #192). Revisit: when #183 or #184 is picked up, or if people who pray at home find X for "alone" harsh.

## 0048 (2026-11-20): Prayer on time and in congregation stay two deeds in v2
Praying on time and praying in congregation stay separate deeds, with no graded single prayer deed. The reports block from 0041 is renamed "The five prayers". For each prayer it shows the owner's on time deed from the five prayers set, or, when there is none, the congregation deed from a level or the mosque prayer set. Owners who already have both deeds keep them and their history as they are; they can archive one in the editor.

Why: a graded deed (congregation is Done, on time alone is half) would score people who rightly pray at home, such as women or people with no mosque near, at half forever. The owner chose not to change the deed shape for v2. Two ways to drop the duplicate rows are parked under the "future feat" label: asking at first run where a person usually prays (#183), and using gender to shape the prayer deeds (#184). Decided by the owner in chat on 2026-11-20 (issue #169, map #104). This revises the block's source in 0041. Revisit: when #183 or #184 is picked up.

Revised by 0049 (2026-11-26): the block uses the congregation deed first and the on time deed only when no congregation deed is due.

## 0047 (2026-11-20): Two new schedules: once a week, and when it comes up
This extends 0009 with two schedules. Every due deed is still due on a day, so scoring stays "mark fractions over due deeds, per day".

**Once a week.** The deed is due on one day of each week, any day the owner picks by marking it. The week runs Saturday to Friday, with no setting for now. Until it has a mark that week, it shows on the sheet every day with a "this week" tag and is open, never Missed. The first day it gets a mark (Done, Partial or Missed), it is due on that day and leaves the sheet for the rest of the week. If Friday ends with no mark that week, it is due on that Friday and counts as Missed. Archiving, deeds made in the middle of a week, late marks and excuse credit follow their usual rules on that one due day.

**When it comes up** (عند الحاجة). The deed shows every day in its section with a quiet tag, but it is due only on a day it has a mark. Done and Partial count as usual, and Missed counts as Missed ("I needed istikhara and did not pray it"). Left unmarked when the day ends, it is not due: it is not Missed, it is not in the section's count, and it does not stop a section from folding by itself (0046).

Level deeds that get these schedules:
* Once a week: visiting graves, and the advanced level's sunan al fitra. Levels 1 to 3 keep sunan al fitra on Fridays.
* When it comes up: istikhara, hajah, the five makeups of missed prayers, and the advanced level's fasting, which names no days.
* Everything else stays as it is, including all situational adhkar (riding too), since they happen almost every day.

Scoring cases the tests must cover:
1. A weekly deed marked Done on Tuesday is due only on Tuesday that week.
2. A weekly deed with no mark all week is due on Friday and Missed.
3. A weekly deed with no mark is open on every day before Friday ends, and today's line counts it as open.
4. A weekly deed marked Missed on Monday is due on Monday and Missed, and is not due again that week.
5. A when it comes up deed with no mark is due on no day, and a day with only such deeds unmarked scores as if they were absent.
6. A when it comes up deed marked Partial or Missed counts that fraction on that day.
7. An excuse credit lifts a weekly deed only on its one due day.

Why: the advanced sheet has deeds due once a week on a day the person picks, and deeds that only apply when a need comes up. Counting those as due every day shows them as missed on days they never applied. Pinning each to one due day keeps the day based model and every existing rule. Rejected: a fourth mark "did not apply" on every deed (changes the mark model for all deeds), and "N times a week" goals (still rejected, as in 0009). Decided by the owner in chat on 2026-11-20 (issue #108, map #104). Revisit: if owners want the week to start on another day, or want a weekly deed more than once a week.

## 0046 (2026-11-20): A shorter daily sheet that still shows the whole day
The daily sheet always shows the whole day; it never hides sections on its own. To make it shorter:
* **One surface.** The sheet is one surface instead of a card per section. Section headings (time slots, groups from 0045, and "any time") are thin and sticky, with a count such as "1 of 3". The today card becomes one line with a thin progress bar.
* **Folding.** Tapping a heading folds or opens its section. A section folds by itself once every deed in it has a mark (#174 builds this part). One button folds or opens all. Each werdy day starts with every section open except those that fold by themselves; folds made by hand do not carry over.
* **Chip strip.** A strip of section chips with counts stays at the top while scrolling. It shows a chip only for sections with deeds that day, scrolls sideways, and has the current prayer's chip outlined and scrolled into view. Tapping a chip jumps to its section and opens it if folded.
* **Smaller buttons, firm floor.** Mark buttons are drawn at about 32px, each with at least a 44px touch area and no overlap; rows stay at least 44px tall. Done sits on the right and missed on the left, with partial or the typed count (0040) between, so a slip cannot land on the opposite mark. A mark is changed by tapping again; there is no undo toast.
* **Text that fits.** Fonts and labels scale with the phone's text size and wrap instead of overlapping.
* **Less on every screen.** The language switch moves from the header to settings. A screen reader reads each row as one deed with its mark buttons inside it.

Measured in the prototype for 23 deeds with nothing folded: today 4.0 phone screens with 3 deeds visible before scrolling; this design about 2.3 screens with 7 to 8 visible, shorter as sections fold.

Why: the levels bring 44 to 113 deeds, which at today's row size is 6 to 16 screens, and friends found the app cluttered (#103). Some people mark at each prayer and some only at night, so nothing may hide part of the day. Rejected: showing only the current slot by default (fails people who mark at night), a single check box with partial and missed behind a menu (hides partial, which the statistics need), and a bigger done button with smaller partial and missed (not taken for now). Brainstormed with the owner on 2026-11-04 with a prototype and settled in chat on 2026-11-20 (issues #73 and #103, map #104). Revisit: if the sheet still feels long at the advanced level once groups and folding are built.

## 0045 (2026-11-20): Groups split "any time" on the daily sheet
A deed sits in one place on the sheet: a time slot or a group. A group holds deeds tied to a kind of deed rather than a time. There are six, a fixed list kept as data like slots and categories: situational adhkar (الأذكار المقيدة), Quran (القرآن), dua (الدعاء), heart and character (القلب والأخلاق), knowledge (العلم) and self review (المحاسبة). The sheet shows the time slots in day order, then the six groups in this order, then "any time" last for deeds that fit neither. Empty groups are hidden, as empty slots are (0007). Categories stay for reports only. A custom deed can pick a group the same way it picks a slot. A group's reminder works like the "any time" reminder: a clock time the owner picks (0038). The Arabic UI word for group is مجموعة.

The level deeds go in groups like this:
* Situational adhkar: the adhkar for waking, the toilet, clothes, wudu, home, the mosque, walking to the mosque, eating, riding and sleep, plus istighfar 100, istighfar in gatherings and kaffarat al majlis.
* Quran: memorising, reading, recitation and reflection, revision and memorisation. The night ward stays in the night slot, where the sheet puts it.
* Dua: for yourself, the teachers, the brothers and all Muslims.
* Heart and character: avoiding the faults of the tongue, lowering the gaze, treating the heart, kindness to parents, family ties, giving people their rights, and dawah.
* Knowledge: attending lessons, reviewing lessons, worldly knowledge.
* Self review: self review and repentance.
* Any time keeps fasting, charity, istikhara, hajah, staying in wudu, wudu before sleep, general istighfar, salawat, study or work, and the Friday Kahf and salawat 100.

The sheets name only some of these headings; heart and character and self review are unnamed blocks on the advanced sheet, and the owner placed those deeds. When the update arrives, werdy moves each level deed that still sits in "any time" to its group, once, from today on. Past days keep their place (0020), and a deed the owner already moved by hand is left alone; the saved starter key (0041) says which deed is which.

Why: the advanced level puts about 51 of its 113 deeds in "any time", one long pile; the level sheets already sort them by kind, and people know those headings. A separate group field beside slot and category was rejected (three axes on every deed), and so was sorting "any time" by category (it would turn the reports list into a sheet tool and grow it). Decided by the owner in chat on 2026-11-20 (issue #107, map #104). Revisit: if owners ask for their own groups, or want a deed in a slot and a group at once.

## 0044 (2026-11-14): Supabase changes deploy from CI on merge
Migrations and Edge Functions deploy to the live project from `.github/workflows/supabase-deploy.yml` after a merge to `main`. CI previews migrations before applying them, then deploys functions through the API. Only trusted main code receives deployment secrets. Deploy runs share a lock and never cancel a running deploy. A missing access token skips functions with a notice; a missing database URL fails migrations. Never edit a migration already on main. Add a new migration that is safe to rerun.

Push setup is a separate manual workflow. It creates VAPID keys only when no public key exists, masks generated values, and keeps the function cron secret and Vault in step. Existing subscriptions keep their keys. Why: the database and functions must follow reviewed code without repeated manual deploy commands, and private push keys must stay out of human hands. Rejected: deploying from pull requests, and rotating push keys on every run. Revisit: when test projects or approval gates are needed before a live deploy.

## 0043 (2026-11-11): Watch requests by email, both ways
A watch request adds a watcher by Google email, in either direction: the owner asks someone to watch them, or a watcher asks to watch an owner. The other person accepts or declines in the app, and accepting creates the same read only share as a share code (0006, 0039). Rules:
* The email is matched after trimming and lowercasing, with no alias rules. Sign in is Google only, so the app asks for "their Google email".
* Sending always answers "Request sent. They see it when they sign in to werdy", so nobody can learn whether an email uses werdy.
* No email is sent. The request waits until a Google account with that email signs in. It lasts 30 days, and the sender can cancel it at any time.
* The receiver sees a card above the sheet and in the sharing section, with the sender's email and what accepting means. At every sign in, waiting requests are shown first with Accept, Decline and Later.
* Declining is quiet: the request leaves the sender's waiting list, the same as when it expires.
* Two requests for the same link (A asks to watch B, and B asks A to watch B) become a share at once.
* At most 5 waiting sent requests per person, one waiting request per pair and direction, none to your own email or to a link that already exists. No cap on watchers or watched people.
* The server keeps the receiver's email only for matching. The request is deleted when it is answered, cancelled or expired, or when the sender or receiver deletes their account.
* Share codes stay, for people sitting together. Revoke and Stop watching are unchanged and tell nobody why.
* First run: near its end it asks once "Do you have a watcher?" (yes leads to sign in and the email field; Not now skips). Its first screen has a small "I'm here to watch someone" link that signs in and shows waiting requests without any deed setup. Where these sit is settled in #121.

Why: family members are often apart, and a 24 hour code needs both people at once. Accepting in the app keeps consent with both people. Not sending mail keeps the free tier and avoids a mail service for now. Decided by the owner in chat on 2026-11-11 (issue #120, map #117). Revisit: when real invite emails are added, or if requests are used to pester people.

## 0042 (2026-11-11): Prayer reminders reach the web app as pushes of times the phone planned
This revisits 0038, which said web delivery needs a server and left it out. The five prayer slot reminders now reach the web app as Web Push, opt in, per device. The phone runs the `werdy-reminders` planner and uploads only the next 7 days of reminders: for each one a UTC time, its stable id and the finished text. A Supabase Cron job calls an Edge Function each minute, which sends the due ones (research for #118, `docs/context/web-push-research.md`). On iPhone this needs werdy added to the home screen. The default delays after the adhan change from zero to Fajr 45, Maghrib 20, and Dhuhr, Asr and Isha 30 minutes, each still 0 to 120.

* **What the server holds.** Per device, one push subscription (endpoint and two keys) and its list of reminder rows. No coordinates, no deed names, no counts, no language setting. A new upload replaces the device's whole list. A row is deleted once sent, and a row more than 15 minutes overdue is deleted unsent. No log of sent pushes is kept. Only the owner can read or write their rows.
* **Location hint.** The uploaded times still hint at roughly which city a person is in, since Maghrib follows longitude and the gap from Fajr to Maghrib follows latitude. One day of times is already enough, so a shorter window would not hide it. The opt in screen says so plainly. The owner accepted this.
* **Refresh.** The phone replans and uploads again on every app open, after a change to settings or location, after each mark, and when the day changes while werdy is open. The last reminder of the window adds a line asking the person to open werdy to keep reminders coming, since the web cannot wake itself and iPhone forbids silent pushes.
* **Done marks.** An upload leaves out a slot with nothing left due, so a slot marked fully Done while online gets no push. The text names only the prayer ("Dhuhr deeds are waiting"), so a push that slips through (offline, or marked on another device) is unneeded but never wrong. The service worker always shows the push, since iPhone revokes permission otherwise.
* **Text.** The phone builds it from Fluent keys in the person's language. Title "werdy"; a tap opens today's sheet.
* **Leaving.** Turning reminders off, or signing out, unsubscribes that device and deletes its subscription and rows. Deleting the account deletes all of them. A 404 or 410 from the push service deletes that subscription and its rows.
* **Per device.** Each phone plans from its own location, so the phone you carry follows your travel. Two phones with reminders on both ring.
* **Free tier pause.** A free project pauses after a week without activity, and whether cron counts is unknown (`___`). A daily GitHub Actions job makes one small request as a keep alive, beside the family's own sync traffic. The build checks once whether a project with only cron running pauses.

Why: the owner does not want locations on the server, and nothing on the web can schedule a local notification. Decided with the owner in #119 on 2026-11-11. Rejected: a country estimate with times computed on the server (too coarse, Maghrib varies by about half an hour across a large country), and a rounded city location (still a stored location, and it reveals no less than the times). Scope: prayer slot reminders only; clock and deed reminders on the web are a later ticket. Revisit: if the free project pauses despite the keep alive, if people often let the 7 days run out, or if a web standard for local scheduled notifications returns.

## 0041 (2026-11-04): The five fard prayers come from the five prayers starter set
The reports page has a "prayers on time" block (#112). It shows the deeds whose starter key is in the `five_prayers` set in `data/starter-sets.toml`, in that set's order. A deed gets a starter key when it is added from a starter set, and the key is kept in the saved data. If the owner has none of the five, the block is hidden. Why: category Prayer is too wide (sunnah, congregation and first row deeds share it), and this needs no new field and no change to saved data or sync. Cost: a prayer deed the owner typed by hand, or a starter prayer deleted and remade by hand, is not in the block. Approved by the owner in chat on 2026-11-04. Rejected for now: a `fard_prayer` field on catalog deeds (would let congregation deeds count, but needs a rule for two deeds on one prayer), and a switch in the deed editor (a new stored field and sync change). Revisit: if owners want congregation deeds or their own typed prayers in the block.

Revised by 0048 (2026-11-20): the block is named "The five prayers" and, for a prayer with no on time deed, shows the congregation deed instead.

## 0040 (2026-11-04): Counted deeds use done, a typed count, and missed
A deed with a target shows three controls: done, a number button and missed. Done stores the target as the count. The number button opens a field to type the count. Missed stores Missed, and a typed 0 is also Missed. A count at or above the target shows and scores as Done, capped at 1, and the real number is kept (120 of 100 stays 120) for later use. A count below the target is partial, worth count over target. The plus and minus buttons from 0011 are removed. Why: friends found the sheet crowded, and one row of three controls matches the other deeds and the 44px touch floor. Typing is faster than many plus taps for counts like 100 tasbeeh. Approved by the owner in chat on 2026-11-04 (issue #73). Revisit: if people miss quick plus one taps for small targets.

## 0039 (2026-10-28): Watchers see day note text
The owner decided that a watcher sees day note text as well as marks, late signs and excuse credits, read only. The note explains late and excused days, and the owner chooses whom to share with. Sharing needs sign in for both people and can be revoked. Location and prayer method still never leave the phone (0010). Without that location a watched view uses the device's civil date for today, and says so. Revisit: if owners ask for finer control over which notes a watcher sees.

## 0038 (2026-10-28): Reminders: when they fire
Prayer slot reminders for Fajr, Dhuhr, Asr, Maghrib and Isha fire at that prayer plus 0 to 120 minutes, with defaults revised by 0042. Morning, night and any_time slot reminders use a clock time picked by the owner. Deed reminders also use a clock time picked by the owner. One slot notification covers the deeds in that slot that are due and not marked Done. A deed reminder fires only if that deed is due and not Done. Partial still needs a reminder. The dated rule, schedule and marks are checked for the werdy day at the notification time, using the Fajr boundary. A delay that crosses midnight keeps its source prayer date in its stable id.

Prayer slot reminders need a saved location. Without one they are skipped and the scheduler returns a flag so the UI can ask for location. Clock reminders still work, using the civil day without location, as the app does. If Fajr does not occur, the civil day is used. If a prayer does not occur at that place and date, only its slot reminder is skipped. Invalid inputs and other calculation failures still return errors. Ramadan Isha uses `isha_in_ramadan`, selected through the owner's dated Hijri offset. Asr uses the calendar's default Shafi rule.

Settings live in the shared JSON, default to empty maps, and survive a round trip. The scheduler ignores reminder ids for unknown or deleted deeds. Scheduling is pure and returns no text. The app builds text from Fluent keys. A horizon of two days covers today and tomorrow, omitting past minutes and keeping the current minute. A clock minute skipped by a time zone change skips only that notification. If midnight is skipped, the day anchor uses midnight expressed with the offset before the gap. Notifications have stable string ids based on source date and slot or deed, so the app can replace them when rescheduling.

Delivery is Android only for now, through offline local notifications with no server. The web cannot schedule these offline without a server. Stage B adds settings and planning only. Stage C adds UI and Android delivery. Why: one pure planner makes timing and dueness testable without a device. Decided by the orchestrator while the owner was out. The owner reviews this decision in the PR. Revisit: owner feedback, web delivery support, or rules for places where a prayer does not occur.

Revised by 0042 (2026-11-11): prayer reminders now also reach the web app by push, and new prayer reminders start at Fajr 45, Maghrib 20, and Dhuhr, Asr and Isha 30 minutes. The rest of 0038 stands.

## 0037 (2026-10-28): All five prayer times, for reminders
`werdy-calendar` now computes sunrise, Dhuhr, Asr, Maghrib and Isha beside Fajr, using the same NOAA solar equations with no dependencies. This revisits 0030 because slot reminders in #22 need all five prayers and sunrise. Asr defaults to Shafi, with a Hanafi option. Dhuhr is solar noon with no extra minutes. Isha follows the method's angle or interval after Maghrib. The caller selects the Ramadan interval and supplies the UTC offset for each date. Missing times return errors. Tests compare Cairo and Makkah with AlAdhan methods 5 and 4 on four dates across the year within 2 minutes, plus Hanafi Asr and Ramadan Isha. Revisit: if published times differ by more than 2 minutes, or reminders need rules for places where a time does not occur.

## 0036 (2026-10-28): Android saves the same JSON as the web, in a private file
On Android, `FileStore` keeps the owner's records as one JSON file, `werdy.json`, in the app's private files folder. It is the same JSON the web keeps in localStorage, through the same `to_json` and `from_json`. A save writes a temporary file and renames it over the old one, so a crash in the middle never leaves half a file. If the files folder cannot be read, the app falls back to memory rather than crash. Why: the data is small (an estimated few hundred KB a year), all scoring runs in Rust on data in memory so SQL queries would go unused, one format means one set of tests, and SQLite is C code that needs cross compiling with the NDK on Windows. Decided by the agent while the owner was out (#12 had asked: SQLite or a JSON file); the owner reviews it in the PR. Rejected for now: SQLite through rusqlite. Revisit: if the file grows to many MB, or if saving one change without rewriting the file is needed. The `Store` trait keeps a later switch inside `werdy-store`.

## 0035 (2026-10-28): Dioxus stays the UI stack on Android, after the sign in spike
The spike for #13 (`spikes/signin-dioxus`) showed that Dioxus 0.7.10 can do Google sign in on Android the native way: the system Credential Manager sheet appears (not a webview page), the Google ID token reaches Rust, and Rust trades it for a Supabase session with `grant_type=id_token` and a hashed nonce. This confirms 0029, so the Tauri 2 track was not built. The cost is small and known: about 120 lines of Kotlin and Gradle for Credential Manager, called through `manganis::ffi`, and about 30 lines of JavaScript for Google Identity Services on the web. Three workarounds are needed today: the MSVC Rust toolchain on Windows, older Android libraries because dx 0.7.10 generates a Kotlin 2.0.20 compiler, and loading the Kotlin class through the activity's class loader (Dioxus issue #5629). Each is listed in the spike's README. Open: the web sign in giving the same Supabase user id is `___` (the button works; the owner still has to complete the sign in), and release builds and a real phone are untested. Revisit: if the web sign in cannot reach the same user, if a release build hits the dx Windows linker bug, or if a Dioxus update breaks the Kotlin bridge.

## 0034 (2026-10-28): Demo data behind ?demo
Adding `?demo` opens a complete sample sheet with history in a separate browser storage key. The app does not read, write or delete real records in this mode. The sample is deterministic and relative to the current werdy day. Why: testers and people seeing werdy for the first time need reports and history to inspect without setting up months of records. Approved by the owner on 2026-10-28. Revisit: if testers need more sample cases.

## 0033 (2026-10-28): A watcher sees the day note text
When sharing is built (#24), a watcher sees the day note text as well as the note sign and the excuse credit, read only. The owner decided this on 2026-10-28. This closes the open question in 0028 and spec story 63g. Why: a watcher needs the owner's explanation to understand a credited day. Revisit: if the owner changes what a watcher may see.

## 0032 (2026-10-28): Umm al Qura for Saudi Arabia
Saudi Arabia defaults to Umm al Qura. Its Fajr angle is 18.5 degrees, and its Isha rule is 90 minutes after Maghrib, or 120 minutes in Ramadan. Tests compare Makkah, Riyadh and Jeddah with published AlAdhan method 4 Fajr times within 2 minutes. The owner approved this on 2026-10-28. Why: the Saudi default now uses the method intended for that country rather than a fallback. Revisit: if published Saudi times differ by more than 2 minutes.

## 0031 (2026-10-27): GitHub Pages hosts the web app
The web app will be served from this repo at `https://ahmed-ahmed16.github.io/werdy/`. The release build uses `werdy` as its base path, and a service worker keeps the app shell for offline visits. Why: GitHub Pages is free, serves over HTTPS, needs no server, and keeps the site with its source. The app stores marks locally first. Cost: Pages cannot run server code for sign in or sync, and Pages settings must use GitHub Actions as the deployment source. Revisit: when sign in and sync change hosting needs, or when the owner adds a custom domain.

## 0030 (2026-10-27): Our own Fajr calculation, Egyptian method by default
Fajr times come from our own small calculation of the sun's position (the NOAA solar equations), in the `werdy-calendar` crate, with no dependencies. Methods are data: Egyptian General Authority (Fajr 19.5 degrees, Isha 17.5) is the default in Egypt, Umm al Qura is the default in Saudi Arabia (0032), and Muslim World League is the default elsewhere. Tests compare Cairo and Alexandria against published times for the Egyptian method (read 2026-10-27 from the AlAdhan API, method 5) within 2 minutes; all were within 1. The caller supplies the UTC offset for each date, so daylight saving is decided outside the crate. Why: Fajr is all werdy needs from prayer times today, the formula is short and testable, it builds for the web, and it avoids a date library. Rejected for now: adhaan and salah (both maintained and MIT, but they pull in a date crate for one number). Revisit: all five prayer times for reminders are added in 0037. Revisit again if official Egyptian tables disagree by more than 2 minutes.

## 0029 (2026-10-27): Dioxus, web first, for the MVP
The owner chose Dioxus 0.7 (all Rust) for the UI, and to ship the web build first so a usable MVP exists within days. The same Dioxus code later builds for Android and desktop. All logic stays in `werdy-core` (and saving in `werdy-store`); the UI crate holds no scoring. So if the Android sign in spike (#13) fails for Dioxus, only the UI crate moves to Tauri 2. Rejected: waiting for the spike before building any UI. Revisit: if the spike shows Dioxus cannot do Google sign in on Android.

## 0028 (2026-10-27): A day note with an optional excuse credit
The owner can attach one free text note to any werdy day ("I was ill", "travelling"). With the note, the owner may choose an excuse credit for that day: none (the default, the day counts as usual), half, or full. The credit lifts every due deed on that day that is not Done to at least that value: Missed or unmarked becomes 0.5 or 1, a Partial keeps the higher of its own fraction and the credit, and Done stays 1. The day keeps its marks and shows a note sign in the sheet, the strip and reports, so a credit is never hidden. A note or credit added more than 2 days late carries the "added later" sign, as marks do (0021). The choice is the owner's conscience; werdy does not check it. Why: a sick person or traveller is rewarded for what they used to do (Bukhari 2996), and an owner who was ill should not see that day as a wall of red; an explicit, visible credit is clearer than leaving such days out. Later versions may add notes per category or per deed. A watcher sees the note text, sign and credit, read only (0033). Rejected: hiding excused days from reports (hides history), a fixed credit with no choice (too rigid). This adds scoring rule 6.

## 0027 (2026-10-27): Supabase Free tier accepted, with our own backups
Follows 0016. The owner accepted Supabase's Free tier for v1 after research on 2026-10-27: 500 MB database, 50,000 monthly active users, the Google provider included, and native Android sign in through an ID token. The two gaps are handled by us: a scheduled `supabase db dump` is our backup (Free has none), and pausing after 7 days of low activity is tolerated because the app works offline while the server sleeps. Every table has row level security, and the client ships only the new publishable key. Conflict order in sync uses the server's time, not the phone's, so a phone with a wrong or changed clock cannot make an old mark win; the offset between phone and server is measured on each sync and a large offset is logged. The Sync module speaks werdy operations, not tables, so moving to werdy's own Rust server later is a contained change. Why: zero cost now, standard Postgres underneath, and Google only sign in means no password hashes to migrate. Revisit: real users' data needing managed backups, unwanted pausing, more than 2 projects, or a change in Supabase's terms.

## 0026 (2026-10-27): Consistency instead of streaks; no badges
This settles the streak question parked in 0012. v1 has no streak counter and no badges. Instead it shows two quiet, private signals: per deed consistency ("done on 5 of the last 7 due days"), counted over due days only, and gentle milestones that count days the owner kept their record (any mark, Missed included), not days of Done. After a gap, the owner sees a short welcome back line, which may carry a hadith or saying on consistency in small deeds; each text is verified and its source named before it ships (`___`, texts to be chosen by the owner), and lives in the Fluent files. werdy does not police careless or false marking: it serves people who want the record, and it judges no one. Why: research gathered on 2026-10-27 shows a visible streak becomes a goal in itself and its loss lowers the behaviour (Silverman and Barasch 2023), expected rewards for completion undermine motivation from within while informational feedback helps (Deci, Koestner and Ryan 1999), and one missed day does not materially affect a habit (Lally et al. 2010). The most beloved deeds are the regular ones even if small (Bukhari 6464, Muslim 783), and badges risk riya, even if shown only to the owner. Rejected: a forgiving streak (still a goal in itself), owner only badges (riya risk), weekly goals (wrong for fard prayers). Revisit: if owners ask for more motivation after v1, starting with the milestone wording, never with leaderboards.

## 0025 (2026-10-27): Translations in Fluent files, one per language per feature
UI text lives in Project Fluent (`.ftl`) files, read with the Rust `fluent` crates, split into namespaces by feature: `locales/<lang>/<feature>.ftl`, for example `locales/ar/sheet.ftl`, `locales/ar/reports.ftl`, `locales/en/sheet.ftl`. Each message has a stable key; code asks for keys, never for text. Adding a language is adding one folder with the same file names; changing wording is editing one small file. A test checks that every language has every key. Why: Arabic has six plural forms (for example عمل، عملان، أعمال، عملًا), and Fluent handles plurals, variables and gender in the text file instead of in code. Rejected: JSON with ICU messages (plural rules inside JSON strings are hard to read), and plain JSON keys (plurals would leak into code). Revisit: if the chosen UI stack forces another format.

## 0024 (2026-10-27): Built for change: tokens, strings and data, never hard coded
The owner expects a lot of user feedback and many changes, so nothing a user might ask to change is hard coded. Colours, type sizes, spacing, radii and touch sizes live in one token file (`docs/design/tokens.md` now, a code module later); light and dark are two sets of the same token names, so a theme change is one file. Every piece of UI text lives in the Arabic and English translation files, keyed by name. Screens are built from a small set of reusable components (mark button, deed row, slot group, period chips, report card, day cell). Categories, time slots, Hijri ranges and starter sets are data, not code. Score bands for day colours (80% and 50%) are named settings, not numbers buried in a screen. Why: a change asked for by users should touch one place, be easy to trace in a diff, and be covered by tests. Revisit: never for the principle.

## 0023 (2026-10-27): Dark mode in v1
werdy ships a light and a dark theme in v1, with a setting of system (the default), light or dark. Both themes use the same mark colours and shapes, adjusted so each still passes contrast (4.5 to 1 for text). Why: many people mark Isha, witr and qiyam at night, and a bright screen in a dark room is unkind. Cost: every screen is checked in both themes, which the design tokens make cheap.

## 0022 (2026-10-27): Phones, tablets and the web, one responsive design
Many Android users in the owner's circle use tablets, so every screen is designed for three widths: phone (one column, bottom tabs), tablet (two panes, such as the daily sheet beside today's summary and the week strip, with a side navigation rail), and a wide web window (the same two panes, centred, with a maximum width). Touch targets stay at least 48 px at every width. Why: a phone layout stretched across a tablet wastes space and moves controls out of reach. Revisit: if usage data shows tablets are rare.

## 0021 (2026-10-27): Any past day can be marked; late marks are flagged
The owner can mark or change any past werdy day. A mark entered more than 2 days after its werdy day carries an "added later" sign that the owner and any watcher can see. A due deed left unmarked when its werdy day ends counts as Missed. Why: forgetting to log is common, and a watcher should still be able to tell real time marks from catch up. Rejected: locking days older than a week (loses real history), and no flag (hides catch up).

## 0020 (2026-10-27): Deed changes apply from today on; past scores never move
Editing a deed's schedule, target, slot or category applies from today forward; past werdy days keep the rules they had then. Archiving a deed stops it being due from today, and its history stays in reports. Deleting a deed with all its marks is a separate action that asks for confirmation. Why: a score the owner saw last month must not silently change. Cost: deeds keep a dated history of their rules. This extends scoring rule 4.

## 0019 (2026-10-27): Starter deed sets are our own writing
v1 offers starter deed sets that fill a new sheet in one tap (for example the five prayers, morning and evening adhkar). We write them ourselves from common practice and from the owner's own deed tracker (shared on 2026-10-27; it is his own sheet, adapted from the Werd program he follows, so only the kinds of deeds are used). Werd's sheet may remind us which kinds of deeds people expect, but its wording, order, sections and points are never copied (0001). Why: keeps the clean room intact. Revisit: only through the academy link, with written permission.

## 0018 (2026-10-27): v1 scope
In v1: owner and watcher roles (a watcher is a parent or mentor, read only), the daily sheet, the deed editor with schedules and targets, the four reports, starter deed sets, reminders (local notifications per deed or time slot, no server needed), offline use, and Google sign in for sync and sharing, on Android and the web. Out of v1: a teacher role, watchers assigning deeds, chat between owner and watcher (v1.5 or v2), the academy link (later, and only with written permission, 0001), payments, subscriptions and ads. Why: a small v1 that the owner and family use daily, which can then be shown to the academy.

## 0017 (2026-10-27): v1 ships on Android and the web; iOS comes last
Android and a web app ship together in v1. The web app serves iPhone users until a native iOS app exists, which is the last and hardest step. Why: family members use both kinds of phone. Consequence: the UI stack must build for Android and the web from one codebase, so the Dioxus versus Tauri 2 spike (0002) also checks a web build and Google sign in on the web, not only on Android.

## 0016 (2026-10-27): A free managed backend until the MVP, behind a seam
Sync, sign in and sharing run on a managed backend's free tier until the MVP is shown to the academy. Supabase (Postgres, Google sign in, row level security) is the first candidate; its current free tier limits and terms are `___` and must be checked before `to-spec` closes. The app reaches the backend only through one sync module, so moving later to werdy's own small Rust server is a contained change. Why: the owner wants zero cost for now. Rejected for now: an own Rust server on a paid VPS (more to build, secure and pay for), and Firebase (strong lock in, poor fit for an open source project). Revisit: at the MVP, for learning and scale.

## 0015 (2026-10-27): Local first; Google sign in only for sync and sharing
werdy works fully offline with no account: deeds and marks live on the phone. Signing in with Google is offered when the owner wants backup across phones or to share with a watcher, and a watcher signs in too. Marks made offline sync when the phone is online again. When two of the owner's phones changed the same mark, the later change wins (one mark per deed per werdy day, so conflicts are small). Why: a deeds tracker must work in a mosque with no signal, and an account should not be the price of trying it. Cost: the data model must merge a local only history into an account on first sign in.

## 0014 (2026-10-27): Arabic first, English second
The interface is designed and built in Arabic, right to left, first. English is the second language, chosen with a toggle in the app (not only by the phone's language). Numbers use Western digits (66%) by default in both languages, for readability; a setting switches the Arabic interface to Arabic Indic digits. Every screen, string and layout is checked in both directions. Why: the owner and his family use Arabic; English opens the project to contributors. Revisit: when a third language is requested.

## 0013 (2026-10-27): Today is shown apart from finished days
Week and month scores cover finished werdy days only and say so ("up to yesterday"). Today has its own line: done, partial and open counts. In the day strip, today is an outlined neutral cell, never red. Why: Werd counts today as fully missed, so numbers drop and jump; this is scoring rule 2. Rejected: counting only marked deeds today, and showing a "best case" second number.

## 0012 (2026-10-27): v1 reports, and werdy is never a competition
v1 ships one reports screen with period chips: 3 days, week, month, 6 months and year. Each period is rolling and ends yesterday (week is the last 7 finished werdy days, year the last 365), so periods are always full and comparable; arrows step back one whole period, with no date pickers. Short periods show a day by day strip, long ones a bar per month, and every period shows a split by category. Also in the screen: what did I miss (missed and partial deeds, worst first), and per deed history (rate over due days only, plus a calendar of marks). The headline score covers all due deeds, worship and life work alike. Principle: werdy is for the owner, and for a watcher who helps them; there are no leaderboards, no comparison between people and nothing public. Streaks are parked, `___`: wanted for v1 only if a form is found that motivates without rewarding careless marking. Revisit streaks before `to-spec` closes.

## 0011 (2026-10-27): A partial mark is a count when the deed has a target
A deed may have a target with a unit, such as 5 pages or 100 tasbeeh. Its mark is then a count entered with plus and minus buttons, and the fraction is the count over the target: 3 of 5 is 0.6, a count of 0 is Missed, and a count at or above the target is Done (capped at 1). A deed with no target has one Partial tap, worth a fixed half. Why: accurate fractions without a slow slider. This settles the open input shape in 0004. The plus and minus input is replaced by 0040.

## 0010 (2026-10-27): A werdy day ends at real Fajr
The day rolls over at the computed Fajr time for the owner's location, not at midnight or a fixed hour. Why: qiyam and witr after midnight belong to the night before, and this is how the owner thinks of the day. How: on first run the owner either allows location once or picks a city, so GPS is never required. The calculation method defaults by country (Egyptian General Authority in Egypt) and can be changed in settings. Location and method stay on the phone and are never sent to the server. Cost: prayer time calculation runs on the device. Revisit: if city based times prove too rough for travellers.

## 0009 (2026-10-27): v1 schedules: daily, weekdays, Hijri ranges
A deed's schedule is one of: every day; given weekdays (Fridays, Monday and Thursday); or a Hijri date range (Ramadan, the last ten nights, the first ten of Dhul Hijjah, Ayyam al Bid). Hijri dates are computed on the device, with a plus or minus one day setting for local moon sighting. Why: these cover every case in Werd's measured bug and the owner's own sheet. Rejected for v1: "N times a week" goals, because the deed would then be due per week, which changes the scoring model.

## 0008 (2026-10-27): Every deed weighs the same in v1
No points. Each due deed counts one, and a partial mark counts its fraction. Why: simpler to read and to test, and it still answers what was done and what was missed. Note: the 44% versus 66% example in `docs/context/werd-behaviour.md` uses Werd's points, so werdy's tests count deeds, not points. Revisit: if the owner wants the main prayers to weigh more (a weight field, default 1, would add it).

## 0007 (2026-10-27): Deeds have a time slot and a fixed category
The sheet is ordered by time slot (Fajr, morning, Dhuhr, Asr, Maghrib, Isha, night, any time), and empty slots are hidden. Each deed also has one category from a fixed v1 list (Prayer, Quran, Adhkar, Fasting, Charity, Study, Family, Health, Work, Other) for reports. Friday, Ramadan and the last ten nights are schedules, not sections. Why: the sheet follows the day, which fixes Werd's long page of 15 sections, while categories keep the owner's familiar groups. Custom categories come later.

## 0006 (2026-10-27): v1 is for the owner and family, each on their own phone
Each person has their own account and phone. A watcher (usually a parent) sees another person's sheet and reports, read only. The owner of the data creates a share code, the watcher enters it, and the owner can revoke it. Why: the owner wants family use, and consent belongs to the person whose data it is. Cost: v1 needs sign in and a sync server. Later versions let a watcher assign deeds.

## 0005 (2026-10-27): Design before code
The UI is one of the two main complaints about the existing app, so screens are designed (with Claude Design) and agreed before the first line of UI code.

**Approved by the owner on 2026-10-27:** the v1 design canvas, https://claude.ai/artifact/WmyN8xF2sjWMGV4AJYTk5y. It holds the daily sheet (Arabic, English and dark), the deed editor, My deeds, settings and sharing, reports for week, month and 6 months, a tablet layout, and the design sheet of tokens and components. The tokens are written down in `docs/design/tokens.md`, and the UX reasoning in `docs/context/ux-research.md`. The current colours stay; an academy themed colourway is parked as an idea (`.scratch/ideas/academy-theme.md`).

## 0004 (2026-10-27): Three state marks per deed
Each mark is Done, Partial (a fraction) or Missed. Scores sum the fractions. Why: real life is not black and white, and binary checkmarks hide progress. Revisit: never for the core idea. The partial input shape is settled in 0011.

## 0003 (2026-10-27): Dual MIT OR Apache 2.0, copyright Ahmed Hossam
Anyone can use and contribute; the copyright notice stays on every copy. This is the Rust ecosystem norm.

## 0002 (2026-10-27): Rust
For the owner to learn it, because it works well with AI agents (strict compiler, strong tests), and because the scoring logic fits a small pure crate. Open: the UI stack (Dioxus, all Rust, versus Tauri 2, Rust core with a web UI). The deciding test is a one day spike: Google sign in on Android. `___` until that spike runs.

## 0001 (2026-10-27): Independent clean room app
werdy is not a fork of Werd. We study Werd's behaviour only; we never copy its code or call its backend. A link to the academy comes later, only with the academy's written permission.
