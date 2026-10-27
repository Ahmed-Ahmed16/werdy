reports-title = التقارير
reports-periods = فترة التقرير
reports-three-days = 3 أيام
reports-week = أسبوع
reports-month = 30 يومًا
reports-six-months = 6 أشهر
reports-year = سنة
reports-previous = الفترة السابقة
reports-next = الفترة التالية
reports-range = من { $startDay } { $startMonth } { $startYear } إلى { $endDay } { $endMonth } { $endYear }
reports-until-yesterday = حتى أمس
reports-lead-missed = أكثر ما فاتك: { $deed }.
reports-lead-all-done = أتممت كل الأعمال المستحقة.
reports-due-deeds =
    { $due ->
        [zero] { $due } أعمال مستحقة
        [one] عمل واحد مستحق
        [two] عملين مستحقين
        [few] { $due } أعمال مستحقة
        [many] { $due } عملًا مستحقًا
       *[other] { $due } عمل مستحق
    }
reports-cell-due-deeds =
    { $due ->
        [zero] { $due } أعمال مستحقة
        [one] عمل واحد مستحق
        [two] عملان مستحقان
        [few] { $due } أعمال مستحقة
        [many] { $due } عملًا مستحقًا
       *[other] { $due } عمل مستحق
    }
reports-due-days =
    { $due ->
        [zero] { $due } أيام مستحقة
        [one] يوم واحد مستحق
        [two] يومين مستحقين
        [few] { $due } أيام مستحقة
        [many] { $due } يومًا مستحقًا
       *[other] { $due } يوم مستحق
    }
reports-missed-due-days =
    { $days ->
        [zero] { $days } أيام مستحقة غير مكتملة
        [one] يوم واحد مستحق غير مكتمل
        [two] يومان مستحقان غير مكتملين
        [few] { $days } أيام مستحقة غير مكتملة
        [many] { $days } يومًا مستحقًا غير مكتمل
       *[other] { $days } يوم مستحق غير مكتمل
    }
reports-score-due = من { reports-due-deeds }
reports-percent = { $percent }%
reports-previous-score = الفترة السابقة: { $percent }%، من { reports-due-deeds }
reports-previous-empty = لم تستحق أعمال في الفترة السابقة.
reports-previous-unavailable = تعذر حساب الفترة السابقة.
reports-series = الأيام والأشهر
reports-today = اليوم، جارٍ
reports-today-short = اليوم
reports-today-counts = { $done } تم، { $partial } جزئي، { $open } مفتوح، { $missed } فائت
reports-open-today = افتح اليوم
reports-category = حسب الفئة
reports-missed = ما فاتني
reports-missed-days = { reports-missed-due-days }
reports-none-missed = لم يفتك عمل في هذه الفترة.
reports-deeds = كل عمل، في الأيام المستحقة فقط
reports-deed-due = من { reports-due-days }
reports-category-due = من { reports-due-deeds }
reports-cell-due = { reports-cell-due-deeds }
reports-not-due = غير مستحق في هذه الفترة، فلا يحسب
reports-empty = لم تستحق أعمال في هذه الفترة. افتح اليوم لترى ما يستحق الآن.
reports-error = تعذر حساب هذا التقرير.
reports-no-score = لا أعمال مستحقة
reports-day-value = { $day } { $month }: { $percent }%، من { reports-due-deeds }
reports-month-value = { $month } { $year }: { $percent }%، من { reports-due-deeds }
reports-day-no-due = { $day } { $month }: لا أعمال مستحقة
reports-month-no-due = { $month } { $year }: لا أعمال مستحقة
reports-history-day = { $day } { $month }
reports-history-label = { $date }: { $mark }
reports-detail-label = { $base } · { $detail }
reports-history-done = تم
reports-history-partial = جزئي
reports-history-missed = فائت
reports-note = ملاحظة اليوم
reports-note-sign = ✎
reports-late-sign = *
reports-unknown-deed = عمل محذوف
category-prayer = صلاة
category-quran = قرآن
category-adhkar = أذكار
category-fasting = صيام
category-charity = صدقة
category-study = دراسة
category-family = أسرة
category-health = صحة
category-work = عمل
category-other = أخرى
