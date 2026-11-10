## The daily sheet.

sheet-date = { $weekday }، { $day } { $month } { $year }
sheet-hijri-date = { $day } { $month } { $year } هـ
sheet-hijri-unavailable = التاريخ الهجري غير متاح
hijri-month-1 = المحرم
hijri-month-2 = صفر
hijri-month-3 = ربيع الأول
hijri-month-4 = ربيع الآخر
hijri-month-5 = جمادى الأولى
hijri-month-6 = جمادى الآخرة
hijri-month-7 = رجب
hijri-month-8 = شعبان
hijri-month-9 = رمضان
hijri-month-10 = شوال
hijri-month-11 = ذو القعدة
hijri-month-12 = ذو الحجة
sheet-previous-day = اليوم السابق
sheet-next-day = اليوم التالي
sheet-back-to-today = العودة إلى اليوم
sheet-empty = لا أعمال مستحقة في هذا اليوم.

summary-title-today = اليوم حتى الآن
summary-title-past = هذا اليوم
summary-in-progress = اليوم لم ينته بعد، فلا نسبة له حتى يتم.
summary-score = النتيجة: { $percent }%
summary-score-error = النتيجة غير متاحة.
summary-line-today = { $done } تم · { $partial } جزئي · { $last } باقٍ
summary-line-past = { $done } تم · { $partial } جزئي · { $last } فائت

slot-fajr = الفجر
slot-morning = الصباح
slot-dhuhr = الظهر
slot-asr = العصر
slot-maghrib = المغرب
slot-isha = العشاء
slot-night = الليل
slot-any_time = أي وقت
slot-count = { $marked } من { $total }
slot-chip = { $slot } { $marked }/{ $total }
slot-chip-label = { $slot }، عُلّم { $marked } من { $total }
sheet-slots = أوقات اليوم
sheet-fold-all = طي الكل
sheet-open-all = فتح الكل
sheet-open-only = الباقي وحده ({ $count })
sheet-unmarked-only = تعرض الآن الأعمال التي لم تُعلَّم بعد.
sheet-unmarked-none = لم يبق عمل دون علامة اليوم.
sheet-show-all = عرض الكل

mark-done = تم
mark-partial = جزئي
mark-missed = فات
mark-added-later = أضيف لاحقًا
count-progress = { $count } من { $target }
count-edit = أدخل العدد
count-empty = ١٢٣
count-goal = الهدف { $target }
count-invalid = أدخل عددًا صحيحًا من 0 إلى { $max }.

note-button = ملاحظة
note-sign = توجد ملاحظة لهذا اليوم
note-sign-symbol = ✎
note-title = ملاحظة اليوم
note-close = إغلاق الملاحظة
note-text-label = ماذا حدث في هذا اليوم؟
note-credit-label = احتساب العذر
note-credit-none = بدون
note-credit-half = نصف
note-credit-full = كامل
note-credit-none-explain = تبقى النتيجة كما هي. الملاحظة للتذكر فقط.
note-credit-half-explain = يحسب كل عمل مستحق بنصفه على الأقل.
note-credit-full-explain = يحسب كل عمل مستحق كاملًا.
note-empty-error = اكتب ملاحظة أو اختر احتسابًا للعذر.
note-length-error = الملاحظة طويلة جدًا. اختصرها ثم احفظها.
note-save-error = تعذر حفظ الملاحظة.
note-date-error = لا يمكن تغيير ملاحظة هذا اليوم.
note-save = حفظ
note-cancel = إلغاء
note-delete = حذف الملاحظة
sheet-welcome-back = أهلًا بعودتك. ابدأ من يومك هذا.
sheet-saying-source = المصدر: { $source }
sheet-saying = { $text }
sheet-milestone =
    { $days ->
        [one] دوّنت ما فعلت في يوم واحد. شكرًا لعودتك إلى التدوين.
        [two] دوّنت ما فعلت في يومين. شكرًا لعودتك إلى التدوين.
        [few] دوّنت ما فعلت في { $days } أيام. شكرًا لعودتك إلى التدوين.
        [many] دوّنت ما فعلت في { $days } يومًا. شكرًا لعودتك إلى التدوين.
       *[other] دوّنت ما فعلت في { $days } يوم. شكرًا لعودتك إلى التدوين.
    }
sheet-dismiss-milestone = إخفاء
consistency-line =
    { $due ->
        [one] تمّ في { $done } من آخر يوم مستحق.
        [two] تمّ في { $done } من آخر يومين مستحقين.
        [few] تمّ في { $done } من آخر { $due } أيام مستحقة.
        [many] تمّ في { $done } من آخر { $due } يومًا مستحقًا.
       *[other] تمّ في { $done } من آخر { $due } يوم مستحق.
    }
consistency-no-due = لا توجد أيام مستحقة منتهية بعد.
summary-last-week = آخر 7 أيام
summary-open-reports = التقرير كاملًا
