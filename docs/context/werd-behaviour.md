# Werd: observed behaviour

Observed on 2026-10-27 from the public store listings and from the owner's own signed in account, read only. Behaviour only; no code, assets or private details are reproduced here.

## What it is
* "Werd" (ورد), a daily good deeds tracker published by an Islamic studies academy. Android (1K+ installs), iOS and a web app. Sign in with Google, Apple or an email code.
* A user follows a deed sheet (a "program"), either from the academy (levels 1 to 3) or their own.

## Screens
* Four bottom tabs: my daily sheet, programs, home (banners and teachers), chat.
* The daily sheet has 15 sections on one long page: Fajr, morning, Dhuhr, Asr, Maghrib, Isha, evening, Quran, fasting, adhkar, study, other good deeds, Friday, last ten nights of Ramadan, Ramadan.
* Each deed is a checkbox with a points badge (1, 2, 5 or 10 points).
* Reports are one tap away, with two date pickers (from, to) and a percentage ring.

## UI problems seen
* Several seconds of loading spinner on start and on every tab switch.
* Date picker OK sometimes needs a second tap.
* The side menu is clipped at the window edge at some sizes.
* The report ring shows 0% for about a second before the real value.
* One long sheet of 15 sections, many irrelevant on most days.

## The statistics bug (measured)
Werd's maximum for each deed is its points times **every** day in the range, whether or not the deed was due.

Week of 2026-10-21 to 2026-10-27, as shown: 375 of 847 points, 44%.

| Problem | Possible points counted | Should be |
| --- | --- | --- |
| Ramadan and last ten nights deeds, in October | 168 | 0 |
| Friday only deeds, counted every day | 70 | 10 (one Friday) |
| Thursday lesson, counted every day | 35 | 5 |
| Monday and Thursday fasting, counted every day | 28 | 4 |

Counting only due days gives about 375 of 565, **66%**. For the month (1 to 27 October) the Thursday lesson shows 15 of 135 (11%) when the real rate is 15 of 20 (75%). The month total 42.61% is displayed as 42% (truncated). Today, still in progress, counts as fully missed. There is no partial credit.

## Scoring rules for werdy (the first tests)
1. A deed has a schedule (every day, given weekdays, Fridays, a date range such as Ramadan, the last ten nights). It only counts on days it is due.
2. Today is "in progress" until the day ends.
3. Each mark is Done, Partial (a fraction) or Missed; the score sums the fractions.
4. A deed added mid period only counts from the day it was added.
5. Percentages round, never truncate.

Counter rule, decided by the owner on 2026-10-28: zero is a miss (red). A positive count below the target is partial (yellow), worth count divided by target. Reaching or exceeding the target is full (green), capped at one. Typing a count stores the same raw count as tapping plus. Typing zero records a miss; minus from one still clears the mark. Direct entry accepts whole numbers from 0 to 4294967295, the same limit as plus, including Arabic Indic digits. Enter or blur saves; Escape cancels. Invalid input keeps the previous mark and shows a message.
