# werdy product requirements (web MVP)

This document describes what the werdy web app does today, so that people and automated testers (TestSprite) can check it. The full v1 plan is [issue #1](https://github.com/Ahmed-Ahmed16/werdy/issues/1); the words in **bold** come from the glossary in `CONTEXT.md`.

## 1. Product summary

werdy is a tracker for daily Islamic deeds and life work, for one person (the **owner**). Every day the owner marks each **deed** that is **due** as Done, Partial or Missed, and reports show what was done, half done and missed, counting only the days each deed was due.

* Live address: https://ahmed-ahmed16.github.io/werdy/
* Type: a static web app (Rust compiled to WebAssembly, Dioxus). There is no server, no sign in and no account.
* Storage: everything is saved in the browser's local storage under the key `werdy.records`, on this device only. A fresh browser profile starts at first run.
* Demo data: https://ahmed-ahmed16.github.io/werdy/?demo opens werdy with about 120 days of sample history (Cairo, 12 deeds, notes, late marks, a gap, one archived deed) in a separate key, `werdy.demo.records`. Real records are never touched. A banner says "Demo data. Your own records are not touched." with "Reset demo" and "Leave demo".
* Language: Arabic and right to left by default. A button in the header switches to English and back.
* Installable as a web app, and the app shell loads offline after one online visit.

## 2. Users

* **Owner**: the only user in this version. Adds deeds, marks them, writes day notes and reads reports.
* A **watcher** (read only viewer) is planned but not built. See section 9.

## 3. Key terms

* **werdy day**: runs from one Fajr to the next at the owner's place. Before Fajr, the app still shows the previous day. Without a place, the day starts at midnight.
* **Due**: a deed counts on a day only when its schedule includes that day and the deed already existed that day.
* **Mark**: Done (worth 1), Partial (worth a half, or count over target for a deed with a target), Missed (worth 0).
* **In progress**: today, until its Fajr ends it. Open deeds today are never shown as missed and today has no percent.
* **Score**: the sum of mark values over the number of due deeds for a set of finished days, as a rounded percent.

## 4. Screens and flows

### 4.1 First run

Shown when the browser has no saved data.

1. Title "Welcome. Set up your page". The owner chooses a place: "Use my location", or "Choose a city" from a list (Cairo, Riyadh, Makkah, London and others), or "Continue without location". While the browser finds the position, the line "Finding your location. This can take a few seconds." shows and the button is disabled; after 15 seconds without an answer, or if access is refused, "Location was not available. Choose a city instead." shows. Choosing a city while it is finding cancels the lookup.
2. After a city is chosen, a line shows the city and its Fajr time, for example "Cairo, Egypt · Fajr 05:37". A "Prayer time method" choice is shown (Egyptian General Authority by default in Egypt, Umm al Qura in Saudi Arabia, Muslim World League elsewhere).
3. "Continue to starter sets" opens "Start with a ready set". Each **starter set** (for example "The five prayers", "Morning and evening", "Quran") has an Add button; tapping again removes it. More than one set can be added.
4. "Start my page" opens Today with the chosen deeds. "I will add my own deeds" opens an empty sheet.

Expected: after first run, reloading the page opens Today, not first run.

### 4.2 Today (the daily sheet)

* Header: the Gregorian date ("Tuesday, 27 October 2026"), the Hijri date under it ("16 Jumada al Ula 1448 AH"), "Previous day" and "Next day" arrows, a "Note" button and the language switch. "Next day" is disabled on today.
* Summary card "Today so far": counts of Done, Partial and Open, and the line "Today is not over yet, so it has no percent until it ends." On a past day the card is titled "This day" and shows "Score: N%".
* Deeds are grouped by **time slot** in this order: Fajr, Morning, Dhuhr, Asr, Maghrib, Isha, Night, Any time. Empty slots are hidden.
* Each deed without a target has three buttons: Done (check), Partial (half circle), Missed (cross). Tapping a button sets that mark; tapping the same button again clears it; tapping another button switches the mark.
* A deed with a target (for example "100 times") shows "0 of 100" with "Add one" and "Take one away" buttons and a Missed button. A count at or above the target counts as Done; a count of 0 counts as Missed.
* When every deed in a slot is Done, the slot folds into one line ("Fajr: all done") with a "Show" button.
* On load, the sheet scrolls to the current time slot, unless a notice (welcome back or milestone) is showing.
* The owner can step back to any past day and mark it. A mark entered more than 2 days after its day shows an "Added later" sign.
* On a past day, a due deed with no mark shows as Missed.

### 4.3 Day note

* "Note" opens a dialog: a text field "What happened this day?" and "Excuse credit" with None, Half or Full, each with a short explanation.
* Save with text or a credit; saving with neither shows "Write a note or choose a credit."
* A saved note shows a note sign (✎) on the day. "Delete note" removes it.
* Excuse credit lifts every due deed that day that is not Done to at least the credit value (Half: at least 0.5, Full: 1), in the score of a finished day. Marks themselves are not changed: the mark buttons, and today's Done, Partial and Open counts, stay exactly as marked. The effect shows only in the score of a past day and in reports.

### 4.4 Consistency, milestones and welcome back

* My deeds and the per deed report show "Done on X of the last N due days" for each deed (N is 7), or "No finished due days yet."
* A milestone notice appears on Today when the owner has kept a full record (every due deed marked, any mark, Missed included) on 7, 30 or 100 finished days. It has a "Dismiss" button, and a dismissed milestone does not come back.
* After a gap of 3 or more finished days with no marks, following earlier marks, Today shows "Welcome back. Pick up where you are today." It shows once per day.
* There are no streak counters and no badges, by design.

### 4.5 My deeds and the deed editor

* My deeds lists active deeds by time slot with their schedule sentence and consistency line, plus "Archived (N)" with a "Restore" button for each archived deed.
* Links: "Add from a ready set", "City and Fajr", "Settings".
* "Add deed" opens the editor: Name, Category (Prayer, Quran, Adhkar, Fasting, Charity, Study, Family, Health, Work, Other), Time slot, "When is it due?" (Every day, Weekdays, Hijri range), optional "Daily target" with a unit (pages, rakaat, times).
* The editor reads the schedule back as a sentence: "Due every day.", "Due every Monday and Thursday.", "Due during Ramadan."
* Errors: an empty name shows "Enter a name."; Weekdays with no day picked shows "Pick at least one weekday."
* Editing a deed applies from today on ("Changes start today. Past days and scores stay the same.").
* "Archive deed" stops the deed being due from today; its past marks still count. "Delete deed and all marks" asks for confirmation ("Delete permanently") first.

### 4.6 Reports

* Period chips: 3 days, Week, 30 days, 6 months, Year. Every period ends yesterday and the screen says "Up to yesterday". Arrows step one whole period back or forward.
* A lead sentence first ("Most missed: <deed>." or "All due deeds were done."), then the score with "Out of N due" and the previous period's score.
* A day by day strip for short periods and a bar per month for long ones. Cells are coloured by score band (good from 80%, middle from 50%); today is outlined, neutral and labelled "Today, in progress".
* Sections: "By category", "What did I miss?" (worst first), and "Each deed, on due days only" with a mark history.
* With no due deeds in the period: "No deeds were due in this period. Open Today to see what is due now." with an "Open Today" button.

### 4.7 Settings

Reached from My deeds (phone) or from the side rail (840 px and wider).

* Language: Arabic or English. Digits: Western digits or Arabic Indic digits. Theme: System, Light or Dark.
* Hijri date adjustment: Minus one day, No change, Plus one day (applies from today).
* City and Fajr method: opens the place screen from first run.
* All settings are kept after a reload.

## 5. Scoring rules (must always hold)

1. A deed counts only on days it is due. A Friday deed is not counted on a Tuesday; a Ramadan deed is not counted in October.
2. Today is never counted as missed and never inside a report period.
3. Percents are rounded, never cut: 42.61% shows as 43%.
4. Partial counts: 3 of 5 pages is worth 0.6; Partial without a target is worth 0.5.
5. Every deed weighs the same. There are no points.
6. Excuse credit raises the value of a due deed that is not Done to at least the credit.
7. A deed added today counts only from today, so past scores never drop when a deed is added.

## 6. Layout and accessibility

* Phone (under 840 px): one column, bottom tab bar with Today, Reports and My deeds.
* Tablet and desktop (840 px and wider): a navigation rail with Today, Reports, My deeds and Settings; on Today the summary and a "Last 7 days" strip sit beside the sheet; reports use two columns.
* Wide windows: the content is centred with a maximum width of 72rem.
* No sideways scrolling at any width.
* Every mark has its own shape as well as its colour. Buttons are at least 48 px tall. Keyboard focus is visible and every button can be reached with the keyboard.
* Arabic is right to left, English is left to right, and the layout mirrors.

## 7. Data and privacy

* No network calls for user data. Place and prayer method stay on the device.
* Data lives only in this browser's local storage. Clearing site data resets the app to first run.
* If saved data cannot be read, the app shows an error and does not write over it.

## 8. Suggested test scenarios

Limits of a fresh browser profile, which every automated test starts from:

* A deed counts only from the day it was added (rule 7), and reports end yesterday. So on a fresh profile every report says "No deeds were due in this period", past days have no due deeds, and "Added later" cannot appear. This is correct behaviour, not missing data. **Test reports, scores, month bars, "What did I miss?", consistency, milestones, excuse credit effects and late marks at `?demo`**, which has that history.
* "Use my location" needs the browser to grant location access. If it is refused, or nothing answers within 15 seconds, the app shows "Location was not available. Choose a city instead." The chosen place is shown on the place screen, not on Today.
* "Delete deed and all marks" removes a deed for good; it does not move it to Archived. Only "Archive deed" does that.
* Layout checks need the test browser to resize its window.

1. Fresh browser: first run appears; choose Cairo; the Fajr time line appears; add "The five prayers"; Start my page; Today shows 5 prayer deeds and "Today so far" with 5 open.
2. Mark Fajr Done: the Done count becomes 1 and Open drops by 1. Tap Done again: the mark clears.
3. Switch to English and back: all labels change language and direction; marks are kept.
4. Reload the page: the sheet, marks and language are kept.
5. Go to the previous day, mark a deed, return with "Back to today".
6. Add a deed due on Monday and Thursday: the editor shows "Due every Monday and Thursday."; on other days it is not on the sheet.
7. Save the editor with an empty name: "Enter a name." appears and nothing is saved.
8. Add a deed with a target of 5 pages; tap "Add one" 3 times: it shows "3 of 5".
9. Write a day note with Half credit: the note sign appears.
10. Reports on a new profile: the empty message and "Open Today" button appear; today is never counted.
11. Settings: switch to Dark theme and Arabic Indic digits; both apply at once and survive a reload.
12. Resize to 375 px, 768 px, 1024 px and 1600 px: no sideways scroll; bottom bar under 840 px, side rail from 840 px.
13. Open `?demo`: the demo banner shows; Reports for Week, 30 days and 6 months show a score with "Out of N due", a lead sentence naming the most missed deed, and a "What did I miss?" list; the strip shows a note sign on a credited day.
14. At `?demo`, go back to a day from about 8 days ago: some marks carry the "Added later" sign. The 100 day milestone line shows on Today.
15. At `?demo`, tap "Reset demo": the sample history is rebuilt for today. "Leave demo" opens the normal app, with its own records unchanged.

## 9. Out of scope for this build

Not built yet, so not to be tested: Google sign in, accounts and sync between devices, sharing with a watcher, reminders and notifications, backups, the Android app, a native iOS app. Also out by design: streak counters, badges, points, leaderboards, custom categories, "N times a week" schedules.
