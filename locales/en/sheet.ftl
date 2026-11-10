## The daily sheet.

sheet-date = { $weekday }, { $day } { $month } { $year }
sheet-hijri-date = { $day } { $month } { $year } AH
sheet-hijri-unavailable = Hijri date unavailable
hijri-month-1 = Muharram
hijri-month-2 = Safar
hijri-month-3 = Rabi al Awwal
hijri-month-4 = Rabi al Thani
hijri-month-5 = Jumada al Ula
hijri-month-6 = Jumada al Akhira
hijri-month-7 = Rajab
hijri-month-8 = Shaban
hijri-month-9 = Ramadan
hijri-month-10 = Shawwal
hijri-month-11 = Dhul Qadah
hijri-month-12 = Dhul Hijjah
sheet-previous-day = Previous day
sheet-next-day = Next day
sheet-back-to-today = Back to today
sheet-empty = No deeds are due on this day.

summary-title-today = Today so far
summary-title-past = This day
summary-in-progress = Today is not over yet, so it has no percent until it ends.
summary-score = Score: { $percent }%
summary-score-error = Score unavailable.
summary-line-today = { $done } done · { $partial } partial · { $last } open
summary-line-past = { $done } done · { $partial } partial · { $last } missed

slot-fajr = Fajr
slot-morning = Morning
slot-dhuhr = Dhuhr
slot-asr = Asr
slot-maghrib = Maghrib
slot-isha = Isha
slot-night = Night
slot-any_time = Any time
slot-count = { $marked } of { $total }
slot-chip = { $slot } { $marked }/{ $total }
slot-chip-label = { $slot }, { $marked } of { $total } marked
sheet-slots = Slots of the day
sheet-fold-all = Fold all
sheet-open-all = Open all
sheet-open-only = Only open ({ $count })
sheet-unmarked-only = Showing deeds with no mark yet.
sheet-unmarked-none = Every deed today has a mark.
sheet-show-all = Show all

mark-done = Done
mark-partial = Partial
mark-missed = Missed
mark-added-later = Added later
count-progress = { $count } of { $target }
count-edit = Enter count
count-empty = 123
count-goal = Goal { $target }
count-invalid = Enter a whole number from 0 to { $max }.

note-button = Note
note-sign = Day note
note-sign-symbol = ✎
note-title = Day note
note-close = Close note
note-text-label = What happened this day?
note-credit-label = Excuse credit
note-credit-none = None
note-credit-half = Half
note-credit-full = Full
note-credit-none-explain = The score stays as marked. This note helps you remember.
note-credit-half-explain = Each due deed counts for at least half.
note-credit-full-explain = Each due deed counts in full.
note-empty-error = Write a note or choose a credit.
note-length-error = Note is too long. Shorten it before saving.
note-save-error = Could not save note.
note-date-error = Cannot change a note for this day.
note-save = Save
note-cancel = Cancel
note-delete = Delete note
sheet-welcome-back = Welcome back. Pick up where you are today.
sheet-saying-source = Source: { $source }
sheet-saying = { $text }
sheet-milestone =
    { $days ->
        [one] You kept a record on { $days } day. Thank you for coming back to it.
       *[other] You kept a record on { $days } days. Thank you for coming back to it.
    }
sheet-dismiss-milestone = Dismiss
consistency-line =
    { $due ->
        [one] Done on { $done } of the last { $due } due day.
       *[other] Done on { $done } of the last { $due } due days.
    }
consistency-no-due = No finished due days yet.
summary-last-week = Last 7 days
summary-open-reports = Full report
