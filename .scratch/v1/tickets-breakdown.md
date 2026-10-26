# werdy v1: proposed ticket breakdown (draft, for Ahmed's approval)

Source: `.scratch/v1/spec.md`. Once approved, the spec becomes one parent GitHub issue and each ticket below becomes a sub issue, with GitHub's native "blocked by" links.

Note on slicing: until the spike picks the UI stack, there is no UI to slice through, so tickets 02 to 11 are verified by tests through Core, Calendar and Store alone. From ticket 13 on, every ticket is a full vertical slice through the app.

Labels: `ready-for-agent` unless marked `ready-for-human`.

| # | Title | Blocked by | What it delivers |
| --- | --- | --- | --- |
| 01 | Workspace skeleton and CI | none | A Rust workspace with a Core crate, licences, CI running format, lint and tests, and one passing test, so every later ticket lands green |
| 02 | Score counts only due days | 01 | Core types (deed, schedule, mark, werdy day) and the score over due deeds with fractions and rounding: scoring rules 1, 3 and 5, Werd's measured week rebuilt (deeds, not points), 42.61% shows as 43% |
| 03 | Deed changes never move past scores | 02 | Dated rule history: edits apply from today on, a deed added mid period counts from that day, archived deeds keep their history (rule 4, ADR 0020) |
| 04 | Today is in progress; unmarked past deeds are Missed | 02 | Rule 2, unmarked due deed on a finished day counts 0, late flag at more than 2 days (ADR 0013, 0021) |
| 05 | Targets and counted partial marks | 02 | Count over target, capped at 1, 0 is Missed, a Partial without a target is a half (ADR 0011) |
| 06 | Excuse credit on a day | 02 | Rule 6: a day note with none, half or full credit lifts not Done deeds to at least that value (ADR 0028) |
| 07 | Report builder | 03, 04, 05, 06 | Rolling periods ending yesterday (3 days to a year), "out of N due", day or month series, category split, "what did I miss" worst first, per deed history (ADR 0012) |
| 08 | Consistency and milestones | 04 | "Done on X of the last N due days" per deed and milestone counts of kept days (ADR 0026) |
| 09 | Hijri dates and Hijri schedules | 02 | Hijri date for a day with a plus or minus one day offset, and Hijri range schedules (Ramadan, last ten nights, Dhul Hijjah, Ayyam al Bid, Ashura, Arafah) as data (ADR 0009). Reference data `___` |
| 10 | Fajr and the werdy day boundary | 01 | Fajr time from place and method, moment to werdy day, qiyam after midnight belongs to the night before (ADR 0010). Crate or own code and reference data `___` |
| 11 | Local store round trip | 03 | Deeds with rule history, marks, notes and settings saved and loaded back identical (test seam 5) |
| 12 | Spike: Google sign in on Android and the web | 01 | One day, Dioxus versus Tauri 2, pass criteria from the spec; ends in an ADR naming the UI stack. `ready-for-human` (Ahmed sets up Rust, NDK, Google clients) |
| 13 | Tracer bullet: mark today's five prayers | 10, 11, 12 | The app on Android and the web shows today's sheet from the five prayers set, grouped by slot, marks Done, Partial or Missed, saves locally, Arabic right to left, tokens and Fluent files in place |
| 14 | Deed editor and My deeds | 13, 05, 09 | Add and edit deeds with slot, category, schedule (daily, weekdays, Hijri), target; archive, restore, delete with confirmation; schedule read back as a sentence |
| 15 | First run and starter sets | 13 | Location once or a city, method by country, starter sets from data in one tap, no duplicate deeds when sets overlap. Werd presets look for missing kinds (kinds only) |
| 16 | Reports screen | 13, 07 | Period chips, arrows, day strip with score bands, month bars, category split, missed list, per deed history, "up to yesterday" |
| 17 | Past days, late marks and day notes | 13, 06 | Arrows to earlier days, "added later" sign, a day note with excuse credit, note sign shown everywhere |
| 18 | Consistency and welcome back | 16, 08 | Consistency line per deed, quiet milestones, a welcome back line after a gap. Verified texts `___` from Ahmed |
| 19 | Settings: language, digits, theme, Hijri offset, method | 13 | Arabic and English toggle, Western or Arabic Indic digits, system, light or dark, plus or minus one Hijri day, calculation method |
| 20 | Tablet and wide web layouts | 13 | Two panes and a navigation rail from 840 dp, centred wide web layout, keyboard path and focus rings, landscape and split screen |
| 21 | Reminders | 13, 10 | Local notifications per deed or per time slot, offline |
| 22 | Sign in and sync | 12, 11 | Supabase project with row level security, Google sign in on both platforms, push and pull with server time ordering, first sign in merge, fake backend tests (ADR 0015, 0027) |
| 23 | Sharing with a watcher | 22, 16 | Share code or QR, watcher read only sheet and reports, revoke. Note text visibility `___` |
| 24 | Scheduled backups | 22 | A scheduled `supabase db dump` kept outside Supabase (ADR 0027). `ready-for-human` |
| 25 | Installable web app | 13 | Manifest, icons, HTTPS hosting, works without install. Hosting choice `___` |

Frontier at the start: 01. After 01: 02, 10 and 12 in parallel.
