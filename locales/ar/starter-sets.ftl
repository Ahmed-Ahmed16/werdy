# Starter set names, deed names and target units (ADR 0019, ADR 0025).
# Keys come from data/starter-sets.toml. Our own wording.

## Units. $count is the target amount. Arabic uses all six plural forms.

unit-pages =
    { $count ->
        [zero] صفحة
        [one] صفحة
        [two] صفحتان
        [few] صفحات
        [many] صفحة
       *[other] صفحة
    }
unit-rakaat =
    { $count ->
        [zero] ركعة
        [one] ركعة
        [two] ركعتان
        [few] ركعات
        [many] ركعة
       *[other] ركعة
    }
unit-times =
    { $count ->
        [zero] مرة
        [one] مرة
        [two] مرتان
        [few] مرات
        [many] مرة
       *[other] مرة
    }

## Sets.

set-five-prayers = الصلوات الخمس
set-sunnah-prayers = السنن الرواتب
set-mosque-prayer = الصلاة في المسجد
set-with-every-prayer = مع كل صلاة
set-morning-evening = الصباح والمساء
set-night-prayer = صلاة الليل
set-quran = القرآن
set-daily-adhkar = أذكار اليوم
set-friday = يوم الجمعة
set-knowledge-dawah = العلم والدعوة
set-fasting-seasons = الصيام والمواسم
set-ramadan = رمضان

## The five prayers.

deed-fajr-on-time = صلاة الفجر في وقتها
deed-dhuhr-on-time = صلاة الظهر في وقتها
deed-asr-on-time = صلاة العصر في وقتها
deed-maghrib-on-time = صلاة المغرب في وقتها
deed-isha-on-time = صلاة العشاء في وقتها

## Sunnah prayers.

deed-sunnah-before-fajr = سنة الفجر
deed-sunnah-before-dhuhr = سنة الظهر القبلية
deed-sunnah-after-dhuhr = سنة الظهر البعدية
deed-sunnah-after-maghrib = سنة المغرب
deed-sunnah-after-isha = سنة العشاء

## Prayer in the mosque.

deed-fajr-congregation = الفجر جماعة من أولها
deed-fajr-opening-takbir = تكبيرة الإحرام مع الإمام في الفجر
deed-fajr-first-row = الصف الأول في الفجر
deed-dhuhr-congregation = الظهر جماعة من أولها
deed-dhuhr-opening-takbir = تكبيرة الإحرام مع الإمام في الظهر
deed-dhuhr-first-row = الصف الأول في الظهر
deed-asr-congregation = العصر جماعة من أولها
deed-asr-opening-takbir = تكبيرة الإحرام مع الإمام في العصر
deed-asr-first-row = الصف الأول في العصر
deed-maghrib-congregation = المغرب جماعة من أولها
deed-maghrib-opening-takbir = تكبيرة الإحرام مع الإمام في المغرب
deed-maghrib-first-row = الصف الأول في المغرب
deed-isha-congregation = العشاء جماعة من أولها
deed-isha-opening-takbir = تكبيرة الإحرام مع الإمام في العشاء
deed-isha-first-row = الصف الأول في العشاء

## With every prayer.

deed-fajr-dua-before-iqama = الدعاء بين الأذان والإقامة في الفجر
deed-fajr-adhkar-after = الأذكار بعد صلاة الفجر
deed-dhuhr-dua-before-iqama = الدعاء بين الأذان والإقامة في الظهر
deed-dhuhr-adhkar-after = الأذكار بعد صلاة الظهر
deed-asr-dua-before-iqama = الدعاء بين الأذان والإقامة في العصر
deed-asr-adhkar-after = الأذكار بعد صلاة العصر
deed-maghrib-dua-before-iqama = الدعاء بين الأذان والإقامة في المغرب
deed-maghrib-adhkar-after = الأذكار بعد صلاة المغرب
deed-isha-dua-before-iqama = الدعاء بين الأذان والإقامة في العشاء
deed-isha-adhkar-after = الأذكار بعد صلاة العشاء

## Morning and evening.

deed-morning-adhkar = أذكار الصباح
deed-tahlil = لا إله إلا الله
deed-sitting-until-sunrise = الجلوس حتى الشروق
deed-duha = صلاة الضحى
deed-evening-adhkar = أذكار المساء
deed-istighfar = الاستغفار
deed-sleep-adhkar = أذكار النوم

## Night prayer.

deed-qiyam = قيام الليل
deed-witr = الوتر

## Quran.

deed-quran-reading = قراءة القرآن
deed-quran-memorising = حفظ جديد
deed-quran-review = مراجعة المحفوظ

## Everyday adhkar.

deed-waking-adhkar = ذكر الاستيقاظ من النوم
deed-toilet-adhkar = ذكر دخول الخلاء والخروج منه
deed-wudu-adhkar = ذكر الوضوء
deed-dressing-adhkar = ذكر لبس الثوب
deed-home-adhkar = ذكر دخول البيت والخروج منه
deed-food-adhkar = ذكر الطعام والشراب
deed-riding-adhkar = ذكر الركوب
deed-mosque-adhkar = ذكر دخول المسجد والخروج منه
deed-walking-to-mosque-adhkar = ذكر الخروج إلى المسجد

## Friday.

deed-friday-ghusl = غسل الجمعة
deed-sunan-al-fitra = سنن الفطرة
deed-best-clothes-and-perfume = أحسن الثياب والطيب
deed-siwak = السواك
deed-early-to-jumuah = التبكير إلى الجمعة
deed-surah-al-kahf = سورة الكهف
deed-salawat = الصلاة على النبي ﷺ
deed-friday-last-hour-dua = الدعاء في آخر ساعة من يوم الجمعة

## Knowledge and dawah.

deed-attending-lesson = حضور درس علم
deed-reviewing-lessons = مراجعة الدروس
deed-dawah = الدعوة إلى الله

## Fasting and seasons.

deed-fasting-monday-thursday = صيام الاثنين والخميس
deed-fasting-ayyam-al-bid = صيام الأيام البيض
deed-fasting-ashura = صيام تاسوعاء وعاشوراء
deed-charity-ashura = الصدقة يوم عاشوراء
deed-family-ashura = التوسعة على الأهل يوم عاشوراء
deed-fasting-dhul-hijjah = صيام تسع ذي الحجة
deed-fasting-arafah = صيام يوم عرفة
deed-takbir-dhul-hijjah = التكبير في أيام ذي الحجة

## Ramadan.

deed-ramadan-fasting = صيام رمضان
deed-taraweeh = صلاة التراويح
deed-ramadan-quran = القرآن في رمضان
deed-ramadan-charity = الصدقة في رمضان
deed-last-ten-qiyam = قيام العشر الأواخر
deed-laylat-al-qadr-dua = الدعاء في ليلة القدر
