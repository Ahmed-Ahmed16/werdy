# Store screenshot capture plan

Capture plan only. No screenshots, app builds, device sessions or emulator runs
were performed for #71. Use sample data throughout. Arabic first, then English.
Names below are suggested capture filenames, not existing image files.

## Demo preparation

1. On the web, open the app with `?demo` appended, such as
   `https://ahmed-ahmed16.github.io/werdy/?demo`. Availability of that deployment
   was not checked. The query selects `werdy.demo.records`, separate from real
   browser records. Demo mode does not load or save account state. Use Reset
   demo data for repeatable samples. Keep the demo banner visible.
2. The seed has 120 days of history, twelve deeds, Partial and Missed marks,
   a gap, late marks, an archived deed and one added ten days ago. Notes are four
   days ago for half credit and sixteen days ago for full credit. Dates depend
   on the current demo day, with Cairo as the sample place. Evidence:
   `crates/werdy-app/src/demo.rs`.
3. Android demo store entry: ___. In this branch,
   `crates/werdy-app/src/platform.rs::data_mode` always returns Normal outside
   the web, and Android `open_store` ignores the mode and opens `werdy.json`.
   Do not seed or replace a person's real records. Native capture is blocked
   until an isolated Android demo entry and store exist in an approved build.
   A narrow web capture can review layout but must not be called an Android
   screenshot.
4. Use the same sample day across a language pair. Capture before Fajr changes
   the day, or reset and repeat the pair. Switch language, theme and digits only
   within demo storage. Do not sign in or redeem a share code. Never include
   real notes, email, share codes, tokens or notification content.
5. Choose actual phone and tablet capture sizes from the release device and
   current Console requirements: ___. Proposed groups below cover phone and
   tablet layouts; no pixel dimensions or device compatibility are claimed.

## Phone captures

Take each row in Arabic, then English. Use filenames such as
`phone-01-ar.png` and `phone-01-en.png`.

| Order | Screen and preparation | What the image shows | Caption draft, Arabic then English |
| --- | --- | --- | --- |
| 01 | Today, light theme. In demo only, mark one due prayer Done and another Partial; leave another open. For Quran, enter a count below its target. | Time slot groups, distinct mark controls, counted progress, Today in progress and the day strip. No finished percentage for today. | سجّل يومك كما هو. / Record your day as it is. |
| 02 | Reports, Week. Scroll only enough to include the score and its due count. | Finished days up to yesterday, prior period comparison where present, and the most missed deed sentence. | نتائج تحسب الأيام المستحقة فقط. / Scores count only due days. |
| 03 | Reports, 30 days. Scroll to category and deed breakdowns and What did I miss? | Categories, per deed due counts and missed due days. | راجع ما فعلت وما فاتك. / Review what you did and missed. |
| 04 | Open the day note four days before today. | Sample travel note, half excuse credit and its explanation. Keep the note sign visible if space permits. | دوّن ملاحظة اليوم واحتساب العذر. / Keep a day note and excuse credit. |
| 05 | My deeds, edit the sample Quran reading deed. Do not save changes. | Name, category, time slot, schedule and count target. Avoid the deletion confirmation. | اختر موعد العمل وهدفه. / Choose when a deed is due and its target. |
| 06 | My deeds, open starter sets. Do not change selections unless resetting afterward. | Ready made sets for prayer, Quran and adhkar. | ابدأ بمجموعة جاهزة أو أضف أعمالك. / Start with a set or add your own deeds. |
| 07 | Today, dark theme, using the same marks as 01. | Same records in dark mode, readable mark shapes and labels. | اختر المظهر المناسب لك. / Choose the theme that suits you. |

If the small phone viewport cannot show all content in a row, keep text readable
and split that row into two captures. Do not stitch screens into a false UI.

## Tablet captures

Capture on each tablet class required for the release: ___. Use the wide layout
in landscape and verify portrait separately. Repeat Arabic then English, with
filenames such as `tablet-01-ar.png` and `tablet-01-en.png`.

| Order | Screen and preparation | What the image shows | Caption draft, Arabic then English |
| --- | --- | --- | --- |
| 01 | Today in the wide layout, light theme, sample marks from phone 01. | Navigation rail, daily sheet, time groups and summary using the available width. | صفحتك اليومية على شاشة أكبر. / Your daily sheet on a larger screen. |
| 02 | Reports, 6 months, light theme. | Period controls, month series and due counts. The seed has 120 days, so do not imply six months of records. | راجع النتائج على مدى الشهور. / Review results across months. |
| 03 | Reports, 30 days, dark theme. Scroll if needed. | Category results and per deed history. Include a late sign or note sign where visible. | تفاصيل كل عمل في أيام استحقاقه. / Each deed on its due days. |
| 04 | My deeds with archived deeds expanded, light theme. | Active and archived sample deeds, showing that archive keeps history. | احتفظ بتاريخ الأعمال المؤرشفة. / Keep the history of archived deeds. |

## Capture review and exclusions

* Check Arabic direction, English direction, unclipped labels, readable numbers
  and full controls at the actual device size. Never invent scores; use those
  computed by the demo. Do not advertise screen areas outside the capture.
* Sample notes are seeded as Arabic text even when the interface is English.
  Keep that honest, or edit only the demo note to an English sample and record
  the change. Evidence: `add_demo_notes` in `crates/werdy-app/src/demo.rs`.
* The days eight and thirteen days before today include late evening adhkar
  marks. Use one if a close view of Added later is needed. The full credit note
  sixteen days ago can replace phone 04 if full credit is the chosen example.
* No sharing, sign in or notification screenshots from demo in this ticket.
  Those require a separate approved capture setup and verified sample accounts
  or delivery. Account purpose text still says sharing comes later even though
  the sharing section exists; resolve before capturing that screen.
* Captions are store drafts, not new app UI. Nothing here adds locale keys.
* Current store image limits, minimum counts and required tablet classes: ___.
  Owner checks them before upload. No claim of current Play requirements rests
  on this repo alone.

## Evidence

* Demo isolation and reset: `crates/werdy-app/src/platform.rs`,
  `crates/werdy-app/src/ui/mod.rs` and ADR 0034 in `docs/decisions.md`.
* Views: `crates/werdy-app/src/ui/sheet.rs`, `reports.rs`, `deeds.rs`,
  `note.rs`, `rail.rs` and `first_run.rs` in the same directory.
* Responsive styles: `crates/werdy-app/assets/app.css`.
* Labels and captions' concepts: `locales/ar/sheet.ftl`,
  `locales/en/sheet.ftl`, `locales/ar/reports.ftl`, `locales/en/reports.ftl`,
  `locales/ar/deeds.ftl`, `locales/en/deeds.ftl`,
  `locales/ar/first-run.ftl` and `locales/en/first-run.ftl`.
