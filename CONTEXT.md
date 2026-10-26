# werdy

werdy tracks daily Islamic deeds and life work for one person and shows clear statistics about them: what was done, half done and missed, counting only the days each deed was due.

## People

**Owner**:
The person whose deeds and marks these are. Only the owner adds deeds and marks them.
_Avoid_: user (too vague), student, child

**Watcher**:
A person the owner allowed to see their sheet and reports, read only. In a family, usually a parent.
_Avoid_: parent (a watcher need not be one), follower, supervisor, teacher

**Share code**:
A short code or QR the owner creates to let one watcher in. The owner can revoke it at any time.
_Avoid_: invite, link

## Deeds

**Deed**:
One thing the owner means to do on the days it is due, such as Fajr in the mosque or reading 5 pages of Quran.
_Avoid_: task, habit, item, werd

**Time slot**:
The part of the day a deed belongs to on the sheet: Fajr, morning, Dhuhr, Asr, Maghrib, Isha, night, or any time.
_Avoid_: section, period

**Category**:
What kind of deed it is, from a fixed list: Prayer, Quran, Adhkar, Fasting, Charity, Study, Family, Health, Work, Other. Used to slice reports, not to order the sheet.
_Avoid_: section, tag, group

**Schedule**:
The rule that says on which days a deed is due: every day, given weekdays, or a Hijri date range (Ramadan, the last ten nights, the first ten days of Dhul Hijjah, Ayyam al Bid).
_Avoid_: frequency, recurrence, repeat

**Due**:
A deed is due on a werdy day when its schedule includes that day and the deed already existed that day. Only due deeds count.
_Avoid_: active, scheduled, applicable

**Archived deed**:
A deed that is no longer due from the day it was archived. Its past marks still count in reports for the days it was due.
_Avoid_: deleted, removed, inactive

**Starter set**:
A ready made group of deeds, written by werdy, that fills a new sheet in one tap.
_Avoid_: program, template, pack

**Target**:
An optional amount with a unit that a deed aims for each due day, such as 5 pages or 100 tasbeeh.
_Avoid_: goal, quota, amount

## Days and marks

**werdy day**:
A day that runs from one Fajr to the next, at the owner's location. Qiyam and witr after midnight belong to the day before.
_Avoid_: date, calendar day

**Daily sheet**:
The list of deeds due on one werdy day, grouped by time slot.
_Avoid_: program, checklist, page

**Mark**:
The owner's record of how a due deed went on one werdy day: Done, Partial or Missed. Each mark is worth a fraction from 0 to 1.
_Avoid_: check, tick, status

**Late mark**:
A mark entered more than 2 days after its werdy day. It counts like any mark and carries an "added later" sign.
_Avoid_: backfill, edit

**Missed**:
A mark worth 0. A due deed still unmarked when its werdy day ends is Missed.
_Avoid_: failed, skipped

**Partial**:
A mark worth more than 0 and less than 1: the count over the target for a deed with a target, or a fixed half for a deed without one.
_Avoid_: half done, in progress

**In progress**:
The state of the current werdy day until its Fajr ends it. Unmarked deeds on an in progress day are open, not missed.
_Avoid_: pending, incomplete

## Reports

**Score**:
For a set of werdy days, the sum of mark fractions over the number of due deeds, as a rounded percentage. Worship and life work deeds count the same; a report can split the score by category.
_Avoid_: points, progress, rate
