reports-title = Reports
reports-periods = Report period
reports-three-days = 3 days
reports-week = Week
reports-month = 30 days
reports-six-months = 6 months
reports-year = Year
reports-previous = Previous period
reports-next = Next period
reports-range = { $startDay } { $startMonth } { $startYear } to { $endDay } { $endMonth } { $endYear }
reports-until-yesterday = Up to yesterday
reports-lead-missed = Most missed: { $deed }.
reports-lead-all-done = All due deeds were done.
reports-due-deeds =
    { $due ->
        [one] { $due } due deed
       *[other] { $due } due deeds
    }
reports-cell-due-deeds = { reports-due-deeds }
reports-due-days =
    { $due ->
        [one] { $due } due day
       *[other] { $due } due days
    }
reports-missed-due-days =
    { $days ->
        [one] { $days } due day not done
       *[other] { $days } due days not done
    }
reports-score-due = Out of { reports-due-deeds }
reports-percent = { $percent }%
reports-previous-score = Previous period: { $percent }%, out of { reports-due-deeds }
reports-previous-empty = No deeds were due in the previous period.
reports-previous-unavailable = Could not calculate the previous period.
reports-series = Days and months
reports-today = Today, in progress
reports-today-short = Today
reports-today-counts = { $done } done, { $partial } partial, { $open } open, { $missed } missed
reports-open-today = Open Today
reports-category = By category
reports-missed = What did I miss?
reports-missed-days = { reports-missed-due-days }
reports-none-missed = No missed deeds in this period.
reports-deeds = Each deed, on due days only
reports-deed-due = Out of { reports-due-days }
reports-category-due = Out of { reports-due-deeds }
reports-cell-due = { reports-cell-due-deeds }
reports-not-due = Not due in this period, so not counted
reports-empty = No deeds were due in this period. Open Today to see what is due now.
reports-error = Could not calculate this report.
reports-no-score = No deeds due
reports-day-value = { $day } { $month }: { $percent }%, out of { reports-due-deeds }
reports-month-value = { $month } { $year }: { $percent }%, out of { reports-due-deeds }
reports-day-no-due = { $day } { $month }: no deeds due
reports-month-no-due = { $month } { $year }: no deeds due
reports-history-day = { $day } { $month }
reports-history-label = { $date }: { $mark }
reports-detail-label = { $base } · { $detail }
reports-history-done = Done
reports-history-partial = Partial
reports-history-missed = Missed
reports-note = Day note
reports-note-sign = ✎
reports-late-sign = *
reports-unknown-deed = Deleted deed
category-prayer = Prayer
category-quran = Quran
category-adhkar = Adhkar
category-fasting = Fasting
category-charity = Charity
category-study = Study
category-family = Family
category-health = Health
category-work = Work
category-other = Other
