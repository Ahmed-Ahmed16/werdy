# Spec: werdy v1

Labels: `ready-for-agent` (once the open items below are closed). Draft; the GitHub repo `Ahmed-Ahmed16/werdy` does not exist yet, so this lives in `.scratch/` (`docs/agents/issue-tracker.md`).

Sources: `CONTEXT.md` (glossary), `docs/decisions.md` (ADRs 0001 to 0025), `docs/context/werd-behaviour.md`, `docs/context/ux-research.md`, `docs/context/android-testing.md`, `docs/design/tokens.md`, and the approved design canvas (ADR 0005). Terms in **bold** below are glossary terms.

## Open items (`___`) to close before tickets

These are not guessed. Each one blocks only the parts named.

1. **Streaks** (ADR 0012): researched on 2026-10-27 (session notes, `streaks-research.md`). Evidence: a visible unbroken streak raises the behaviour and a broken one lowers it, and keeping the streak becomes a goal in itself (Silverman and Barasch 2023); expected rewards for completing a task undermine intrinsic motivation while informational feedback helps (Deci, Koestner and Ryan 1999); badges show small, unstable effects and people ease off once a badge is earned (Sailer and Homner 2020, Anderson et al. 2013); one missed opportunity did not materially affect habit formation (Lally et al. 2010, abstract checked). Islamic inputs: the most beloved deeds are the regular ones even if small (Bukhari 6464, Muslim 783); deeds are by intentions (Bukhari 1); a sick person or traveller is rewarded for what he used to do (Bukhari 2996). Options: A, rolling "done on X of the last N due days" per deed (low risk of careless marking, recommended baseline); B, a private forgiving streak, off by default; C, weekly goals for optional deeds; D, quiet private milestones counting days logged, Missed included. **Decided on 2026-10-27: A plus D, no badges, no streak counter, a welcome back line with a verified saying on consistency (ADR 0026).** The excused case is settled separately as a day note with an optional excuse credit (ADR 0028).
2. **Supabase free tier** (ADR 0016): researched on 2026-10-27 (full notes in the session notes folder, `supabase-free-tier.md`). Free tier: 500 MB database, 1 GB files, 5 GB uncached egress, 50,000 monthly active users, Google provider included, native Android sign in through `signInWithIdToken` documented, 2 active free projects. Gaps: a project pauses after 7 days of low activity (data kept, resumed from the dashboard); no backups on Free. Terms: nothing found that forbids an open source family app; the publishable key is safe in client code only if every table has row level security; the old anon key is being retired by the end of 2026, so use the publishable key. No official Rust client: call the PostgREST and Auth HTTP APIs directly behind the Sync module's interface. Still `___`: the exact pause activity threshold, free tier region choice, and whether the DPA covers Free accounts. **Decided on 2026-10-27: accepted, with a scheduled `supabase db dump` as our backup (ADR 0027).**
3. **Starter sets** (ADR 0019): first draft written from Ahmed's two PDFs on 2026-10-27, `.scratch/v1/starter-sets.md`. Still to do: a look at Werd's presets for missing kinds of deeds (kinds only, no wording, order or points), which needs the Werd web address or Ahmed signed in. Blocks: only the starter set data ticket.
4. **Google sign in on Android and the web** (the "small native Kotlin call" is parked for a later talk with Ahmed) (ADR 0002, 0015, 0017): researched on 2026-10-27 (session notes, `google-sign-in.md`). Android: Credential Manager with Sign in with Google; two Google clients (an Android client with package name and debug and release SHA-1, and a Web client whose ID is the token audience); the old Google Sign-In is deprecated. Google blocks its sign in page inside embedded webviews, which hits both Tauri 2 and Dioxus mobile (both draw in a WebView), so both need a native Kotlin call or the system browser with a deep link. Web: the Google Identity Services button, or the Supabase redirect with PKCE. Session: one HTTP call, `POST /auth/v1/token?grant_type=id_token`, which the Rust core can make with `reqwest` on every platform. Tauri 2: a community plugin (`tauri-plugin-google-auth`, small, MIT) or a small Kotlin plugin of our own. Dioxus 0.7: Rust can call Kotlin since 0.7.4, but async callbacks from Kotlin back to Rust are `___`, the main risk. **Still decided by the spike**, with these pass criteria: on Android the system account sheet appears (not a webview page) and a Supabase user and token come back; on the web the same Supabase user id comes back from localhost; the amount of non Rust code and every workaround is logged.
5. **Font**: closed on 2026-10-27. IBM Plex Sans Arabic, as in the approved design. Noto Sans Arabic is no longer compared.
6. **Hijri date display**: closed on 2026-10-27, as in the approved design. The Hijri date shows on the daily sheet header, on the line under the Gregorian date ("Today · Hijri date"). The design leaves the exact wording as a placeholder, so the format (day, month name, year) and its digits follow the digit setting (ADR 0014); the month names live in the Fluent files.
7. **Smaller unknowns**, kept open for now by Ahmed's call on 2026-10-27: `content-max` width (tokens); NDK version and CLI versions (`android-testing.md`); the source table of Hijri dates and prayer times used as test reference data; the prayer time crate, or our own calculation.

## Problem Statement

A Muslim who wants to keep up their daily deeds (prayers on time, Quran, adhkar, fasting, study) and their life work wants to see clearly, day by day, what they did, what they half did and what they missed. The app many people in Ahmed's circle use today, Werd, helps with this but has problems that make the picture hard to trust and slow to reach:

* Its report counts every deed on every day of the range, even when the deed was not due. A Ramadan deed in October, a Friday deed on a Tuesday, all count as missed. Ahmed measured one week at 44% when counting only due days gives about 66%.
* Today, still in progress, counts as fully missed. Percentages are truncated, not rounded. There is no partial credit: reading 3 of 5 pages counts as nothing.
* The sheet is one long page of 15 sections, most of them irrelevant on a given day. Checkboxes are small. Screens wait on a spinner. Marking a forgotten day is awkward. Reports use two date pickers and do not say what to fix.

A parent who wants to encourage a son or daughter has no simple, consent based way to see their sheet.

## Solution

werdy is a clean room, local first deeds tracker for Android, tablets and the web, Arabic first.

* The owner builds a **daily sheet** of **deeds**, each with a **time slot**, a **category**, a **schedule** and an optional **target**, or fills it in one tap from a **starter set**.
* Each **werdy day** runs from Fajr to Fajr at the owner's location. The sheet shows only deeds **due** that day, grouped by time slot, opening at the current slot.
* Each due deed gets a **mark**: Done, **Partial** (a count over the target, or a fixed half) or **Missed**. Any past day can be marked; a **late mark** is flagged.
* Reports count only due deeds, never count today as missed, round instead of truncating, and lead with what was missed. Every number comes from one pure, tested function.
* Everything works offline with no account. Google sign in is offered only for backup across phones and for sharing with a **watcher**, who sees the sheet and reports read only through a **share code** the owner can revoke.

## User Stories

### First run and setup

1. As a new owner, I want the app to work without an account, so that I can try it before deciding to sign in.
2. As a new owner, I want first run to open on the starter sets, so that I do not face an empty sheet.
3. As a new owner, I want to fill my sheet from a starter set in one tap, so that I can start marking today.
4. As a new owner, I want to combine more than one starter set, so that I get the prayers and the adhkar together.
5. As a new owner, I want to start from an empty sheet if I prefer, so that I build my own deeds.
6. As a new owner, I want to either allow my location once or pick a city, so that my werdy day starts at real Fajr without needing GPS.
7. As a new owner in Egypt, I want the prayer time method set to the Egyptian General Authority by default, so that Fajr matches my mosque without settings.
8. As an owner, I want to change the prayer time calculation method, so that Fajr matches where I live.
9. As an owner, I want my location and calculation method kept on my phone, so that they are never sent to any server.
10. As an owner, I want to adjust Hijri dates by plus or minus one day, so that Ramadan deeds follow my local moon sighting.

### The daily sheet

11. As an owner, I want to see only deeds due today, so that I am not scrolling past Friday and Ramadan deeds on other days.
12. As an owner, I want deeds grouped by time slot from Fajr to night, so that the sheet follows my day.
13. As an owner, I want empty time slots hidden, so that the sheet stays short.
14. As an owner, I want the sheet to open at the current time slot, so that I find the deed I just did without scrolling.
15. As an owner, I want a fully done time slot folded into one line, so that what is left stands out.
16. As an owner, I want to see today's done, partial and open counts at the top, so that I know where today stands at a glance.
17. As an owner, I want today shown as in progress, not as a score, so that open deeds never look like failures before the day ends.
18. As an owner, I want qiyam and witr after midnight to belong to the night before, so that my night prayers count on the right day.
19. As an owner, I want to step to yesterday and earlier with arrows beside the date, so that I can mark a day I forgot.
20. As an owner, I want a short line at the bottom listing deeds not due today, so that I know they still exist.

### Marking

21. As an owner, I want to mark a deed Done with one tap on a large button, so that marking right after prayer is quick and one handed.
22. As an owner, I want to mark a deed without a target as Partial with one tap, worth half, so that real life is not all or nothing.
23. As an owner, I want to enter a count with plus and minus buttons for a deed with a target, so that 3 of 5 pages counts as 0.6.
24. As an owner, I want a count at or above the target to count as Done, so that extra effort is never penalised.
25. As an owner, I want a count of zero to count as Missed, so that the meaning of the count is clear.
26. As an owner, I want to mark a deed Missed explicitly, so that I can record it before the day ends.
27. As an owner, I want to undo or change a mark with one more tap, so that a wrong tap is easy to fix.
28. As an owner, I want a due deed I never marked to count as Missed once its werdy day ends, so that the record is complete.
29. As an owner, I want to mark any past day, so that forgetting to log does not lose real history.
30. As an owner, I want marks entered more than 2 days late to carry an "added later" sign, so that I and my watcher can tell catch up from real time marks.
31. As an owner with weak eyesight or colour blindness, I want each mark to have its own shape as well as colour, so that I can tell Done, Partial, Missed and open apart.

### Deeds

32. As an owner, I want to add a deed with a name, time slot, category, schedule and optional target in one screen, so that editing is not clumsy.
33. As an owner, I want sensible defaults in the deed editor, so that adding a simple daily deed takes seconds.
34. As an owner, I want the editor to read the schedule back as a plain sentence, so that I can check it is what I meant.
35. As an owner, I want a deed due every day, so that daily prayers and adhkar are simple.
36. As an owner, I want a deed due on given weekdays, such as Fridays or Monday and Thursday, so that weekly deeds count only on their days.
37. As an owner, I want a deed due in a Hijri range (Ramadan, the last ten nights, the first ten of Dhul Hijjah, Ayyam al Bid), so that seasonal deeds count only in their season.
38. As an owner, I want to give a deed a target and a unit, such as 5 pages or 100 tasbeeh, so that partial effort is measured.
39. As an owner, I want to pick a category from a fixed list, so that my reports can be split by kind of deed.
40. As an owner, I want my changes to a deed to apply from today on, so that past scores never move.
41. As an owner, I want a deed added today to count only from today, so that my past days are not marked down.
42. As an owner, I want to archive a deed, so that it stops being due while its history stays in my reports.
43. As an owner, I want deleting a deed with all its marks to be a separate action that asks first, so that I cannot lose history by accident.
44. As an owner, I want a My deeds screen listing all my deeds, active and archived, so that I can manage them in one place.
45. As an owner, I want to restore an archived deed, so that it is due again from today. (Behaviour assumed from ADR 0020; confirm, `___`.)

### Reports

46. As an owner, I want a reports screen with period chips for 3 days, week, month, 6 months and year, so that I can choose a view without date pickers.
47. As an owner, I want each period to end yesterday and cover only finished werdy days, so that periods are full and comparable.
48. As an owner, I want the report to say "up to yesterday", so that I know today is not in the number.
49. As an owner, I want arrows to step back and forward one whole period, so that I can compare this week with last week.
50. As an owner, I want my score to count only the days each deed was due, so that the number reflects what was asked of me.
51. As an owner, I want percentages rounded, not truncated, so that 42.61% shows as 43%.
52. As an owner, I want every rate to show its "out of N due", so that a small sample is visible.
53. As an owner, I want a day by day strip for short periods, so that I see which days were weak.
54. As an owner, I want a bar per month for long periods, so that I see the trend.
55. As an owner, I want day cells coloured by score band (good at 80%, middle at 50%) with today outlined and neutral, so that the strip reads at a glance.
56. As an owner, I want the score split by category, so that I see if Quran is slipping while prayers are strong.
57. As an owner, I want a "what did I miss" list, worst first, so that the report tells me what to fix.
58. As an owner, I want each report to lead with one plain sentence, so that I understand it before reading numbers.
59. As an owner, I want per deed history with its rate over due days only and a calendar of marks, so that I see how one deed is going.
60. As an owner, I want archived deeds to still count in reports for the days they were due, so that history is not rewritten.
61. As an owner, I want reports to appear at once with real numbers, so that I never see 0% before the real value.
62. As an owner, I want an empty report to link back to Today, so that I know what to do.
63. As an owner, I want no leaderboards and no comparison with other people, so that werdy stays between me and Allah, and my watcher if I choose.
63a. As an owner, I want each deed to show "done on X of the last N due days", so that I see my consistency without an all or nothing counter.
63b. As an owner, I want quiet private milestones for days I kept my record, Missed included, so that keeping a true record is what is recognised.
63c. As an owner coming back after a gap, I want a short welcome back line with a saying on consistency, so that returning feels welcome, not like a wall of red.

### Day notes and excuses

63d. As an owner, I want to add a short note to any day, so that I remember why it went the way it did.
63e. As an owner who was ill or travelling, I want to give that day an excuse credit of none, half or full, so that my record reflects what I would have done.
63f. As an owner, I want an excused day to keep its marks and show a note sign, so that nothing in my history is hidden.
63g. As a watcher, I want to see that a day has an excuse and its credit, so that I read the record correctly. (Whether the note text is shown is `___`.)

### Reminders

64. As an owner, I want to set a reminder on a deed, so that I do not forget it.
65. As an owner, I want to set a reminder on a time slot, so that one notification covers all its deeds.
66. As an owner, I want reminders to work offline as local notifications, so that they need no server.

### Sign in, sync and sharing

67. As an owner, I want to sign in with Google only when I want backup or sharing, so that an account is never the price of trying.
68. As an owner, I want my offline history merged into my account on first sign in, so that nothing I marked before is lost.
69. As an owner with two phones, I want marks to sync in the background, so that both phones agree without waiting on a spinner.
70. As an owner with two phones, I want the later change to win when both changed the same mark, so that conflicts resolve simply.
71. As an owner, I want marks made offline in the mosque to sync when I am online again, so that no signal is never a problem.
72. As an owner, I want to create a share code or QR for one watcher, so that I decide who sees my sheet.
73. As an owner, I want to see who watches me and revoke any of them, so that consent stays mine.
74. As a watcher, I want to sign in and enter a share code, so that I can see the owner's sheet and reports.
75. As a watcher, I want a read only view, so that I can encourage without changing anything.
76. As a watcher, I want to see the "added later" sign on late marks, so that I understand the record.
77. As a watcher, I want access to end when the owner revokes it, so that the owner's choice is respected.
78. As a watcher of more than one person, I want to switch between the people I watch, so that a parent can follow each child. (Implied by ADR 0006; confirm, `___`.)

### Language, theme and layout

79. As an Arabic speaker, I want the app in Arabic, right to left, by default, so that it feels native.
80. As an English speaker, I want an in app toggle to English, so that I am not bound to my phone's language.
81. As an owner, I want Western digits by default and a setting for Arabic Indic digits, so that numbers read the way I like.
82. As an Arabic speaker, I want correct plural forms, so that counts read naturally.
83. As an owner, I want a dark theme and a system default, so that marking Isha and qiyam at night is easy on the eyes.
84. As a tablet user, I want a two pane layout with a navigation rail, so that the sheet and today's summary sit side by side.
85. As a web user, I want the same two pane layout centred with a maximum width, so that it reads well on a wide window.
86. As an iPhone user, I want the web app to work in my browser and be installable, so that I can use werdy before an iOS app exists.
87. As a keyboard user on a tablet or the web, I want a visible focus ring and a keyboard path to every mark button, so that I can mark without touch.
88. As a tablet user, I want werdy to work in landscape and split screen, so that it is never letterboxed.

### Contributors and the project

89. As a contributor, I want every number traced to one pure, tested function, so that a wrong number is caught by a test.
90. As a contributor, I want a test that fails if any language misses a translation key, so that no screen shows a raw key.
91. As a contributor, I want colours and sizes only as named tokens, so that a theme change touches one file.
92. As a contributor, I want categories, time slots, Hijri ranges and starter sets as data, so that adding one is not a code change.
93. As a translator, I want to add a language by adding one folder of Fluent files, so that I need no code.

## Implementation Decisions

### Modules

Proposed as a Rust workspace with a small number of deep modules. Names are working names.

1. **Core** (pure library, no I/O, no clock, no network). Holds the domain model and all scoring. The single most important module.
   * Domain types: **deed** with a dated history of rules (time slot, category, schedule, target, archived from), **mark** (Done, Partial with a fraction, Missed, plus entered at), **werdy day** identifier, **schedule** (every day, weekdays, Hijri range).
   * Due rule: a deed is due on a werdy day when the rule in force that day has a schedule that includes the day, the deed existed that day, and it was not archived by that day.
   * Mark value: Done is 1; a target count is count over target, capped at 1, 0 is Missed; a Partial without a target is a fixed half; an unmarked due deed on a finished day is 0.
   * Late flag: a mark, day note or excuse credit entered more than 2 days after its werdy day.
   * Excuse credit (scoring rule 6, ADR 0028): on a day with a credit of half or full, each due deed's value is the higher of its own value and the credit.
   * Sheet builder: given a werdy day, deeds and marks, returns the daily sheet grouped by time slot (empty slots dropped, fully done slots marked as folded) and today's done, partial and open counts.
   * Report builder: given a period, deeds and marks, returns the score, the "out of N due", the day by day or month by month series, the split by category, the "what did I miss" list worst first, and per deed history. Periods are rolling and end yesterday. Today is never inside a period.
   * Rounding: one function, round half away from zero to a whole percent (the exact half rule is `___`; tests fix whichever is chosen). Never truncate.
   * Consistency: for a deed and a day, done count over the last N due days (N a named setting, default 7). Milestones: count of werdy days with every due deed marked, any mark. Both counted over due days only, both pure functions in Core.
   * Score bands read from named settings (80% and 50%), not literals.
   * Errors are returned, never panics (AGENTS.md rule 4).
2. **Calendar** (pure library). Turns a moment and a place into a werdy day: computes Fajr for the location and method, computes the Hijri date with the plus or minus one day offset, and answers "which Hijri range is this day in". Core receives already computed werdy days and Hijri dates as plain data, so core stays free of astronomy. Whether to use an existing prayer time crate or our own code is `___`.
3. **Store** (local persistence on the device). Saves deeds with their rule history, marks and settings. Exposes load and save in core's types. The concrete database (for example SQLite) is decided in tickets.
4. **Sync** (the one seam to the backend, ADR 0016). Pushes and pulls deeds and marks, merges a local only history into an account on first sign in, resolves same mark conflicts by later change wins, and handles share codes and watcher read access. Only this module knows about Supabase. A fake backend implements the same interface for tests.
5. **Reminders**: schedules local notifications per deed or per time slot, from Calendar's slot times. Platform specific, kept thin.
6. **UI** (Dioxus or Tauri 2, `___` until the spike). Built from the reusable components in ADR 0024 (mark button, deed row, slot group, period chips, report card, day cell). Reads tokens and Fluent messages, never raw values or text. Holds no scoring logic: it shows what Core returns.
7. **Data files**: tokens; Fluent files per language per feature (`<lang>/<feature>.ftl` under a locales folder); categories, time slots, Hijri ranges and starter sets as data read at startup.

### Key rules restated as contracts

* Scoring rules 1 to 5 from `werd-behaviour.md` are Core's contract.
* Deed changes apply from today on; past werdy days use the rule they had then (ADR 0020).
* Every deed weighs the same (ADR 0008). No points.
* A watcher can read, never write (ADR 0006). Enforced by the backend's access rules, not only by the UI.
* Location and calculation method never leave the device (ADR 0010).
* Sync never blocks a screen (`ux-research.md` section 5).

### Backend (subject to open item 2)

* Supabase free tier: Postgres with row level security, Google sign in. Tables hold one owner's deeds, rule history, marks and share grants. One mark per deed per werdy day, so the merge is per row, later `updated at` wins.
* A share grant links a watcher's account to an owner's data, read only, created by redeeming a share code, deleted on revoke.
* Schema details are left to the sync tickets.
* Clocks (ADR 0027): conflict order uses the server's time, not the phone's. The offset between phone and server is measured on each sync and a large offset is logged. werdy cannot stop a person changing their phone's clock, and does not try to; it only refuses to let that clock decide which change wins.
* The Sync interface speaks werdy operations (push my changes since a cursor, pull changes since a cursor, redeem a share code), not table reads and writes, so a later move from Supabase to werdy's own Rust server changes one module.

## Testing Decisions

### What a good test is

A good test drives a module through its public interface and checks what a person would see (a score, a sheet, a list), not how it is computed inside. Tests use glossary words in their names, for example `ramadan_deed_is_not_due_in_october`. A bug fix starts with a failing test (AGENTS.md rule 3).

### Proposed seams (please confirm)

1. **Core's public API: the main seam.** Almost every behavioural test goes here, as plain data in and plain data out: deeds, marks and werdy days in; sheet or report out. Fast, no mocks.
   * First tests: the five scoring rules plus rule 6 (excuse credit), then Werd's measured week rebuilt with werdy's rule (deeds counted, not points), the Thursday lesson month (15 of 20, 75%), and 42.61% rounding to 43%.
   * Then: rule history (edit mid period, archive mid period, add mid period), late flag at exactly 2 and 3 days, target counts (0, below, at, above), unmarked deed on a finished day versus today.
   * Property tests for invariants: a score is always between 0 and 100; adding a non due deed never changes a score; today never changes a period's score.
2. **Calendar's public API.** Moment plus place to werdy day, and date to Hijri date. Tested against a reference table of known Fajr times and Hijri dates (source `___`), including the after midnight qiyam case and the plus or minus one day offset.
3. **Sync's interface with a fake backend.** First sign in merge, later change wins, offline queue, revoke ends watcher access. The real Supabase is checked by a few manual or integration checks, not in every test run.
4. **Store's load and save.** Deeds with their rule history, marks and settings saved and loaded back must be identical, against a real local database file, so past scores can never move through storage (ADR 0020). Added on 2026-10-27; small.
5. **Data file checks.** Every language has every Fluent key; every token name used exists in both themes; text colour pairs pass 4.5 to 1 contrast; starter sets only use known categories and slots.

UI gets no logic tests of its own beyond these, because it holds no logic. Visual checks (both directions, both themes, three widths, landscape and split screen) are a manual release checklist for v1.

### Prior art

None in the repo yet (pre code). The scoring examples in `werd-behaviour.md` are the reference data for the first tests.

## Out of Scope

* A teacher role, watchers assigning deeds, chat between owner and watcher (v1.5 or v2, ADR 0018).
* The academy link and any academy branding or colourway (later, only with written permission, ADR 0001).
* Payments, subscriptions, ads.
* A native iOS app (last, ADR 0017).
* "N times a week" schedules (ADR 0009).
* Deed weights or points (ADR 0008).
* Custom categories (ADR 0007).
* Leaderboards or any comparison between people (ADR 0012).
* werdy's own Rust server (after the MVP, ADR 0016).
* Streak counters and badges (ADR 0026).
* A third language.
* Anything taken from Werd's code, text, assets, backend or data (ADR 0001).

## Further Notes

* Order of work: this spec, then `to-tickets`, then the one day spike for Google sign in on Android and the web, which decides Dioxus versus Tauri 2. Core and Calendar do not depend on the spike, so their tickets can start before it ends.
* The spike needs Rust with the four Android targets, the NDK, the cmdline tools, `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`, and a tablet emulator (which downloads a system image and needs Ahmed's OK). See `docs/context/android-testing.md`.
* Learning: Core's due rule, mark value and rounding function are small and central. They are good pieces for Ahmed to type himself with guidance.
* Public copy about Werd stays respectful: werdy is inspired by it and focuses on different choices, without right versus wrong framing.
* The Lally et al. (2010) claim in `ux-research.md` is still unchecked and must not be used in user facing text.
