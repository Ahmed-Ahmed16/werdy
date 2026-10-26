# Decisions

Newest first. Each entry: the decision, why, and what would make us revisit it.

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
v1 offers starter deed sets that fill a new sheet in one tap (for example the five prayers, morning and evening adhkar). We write them ourselves from common practice and from the owner's own Excel and PDF deed tracker (to be shared, `___`). Werd's sheet may remind us which kinds of deeds people expect, but its wording, order, sections and points are never copied (0001). Why: keeps the clean room intact. Revisit: only through the academy link, with written permission.

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
A deed may have a target with a unit, such as 5 pages or 100 tasbeeh. Its mark is then a count entered with plus and minus buttons, and the fraction is the count over the target: 3 of 5 is 0.6, a count of 0 is Missed, and a count at or above the target is Done (capped at 1). A deed with no target has one Partial tap, worth a fixed half. Why: accurate fractions without a slow slider. This settles the open input shape in 0004.

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
