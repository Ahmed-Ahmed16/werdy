# Starter set names, deed names and target units (ADR 0019, ADR 0025).
# Keys come from data/starter-sets.toml. Our own wording, except the level sets at the end.

## Units. $count is the target amount. Arabic uses all six plural forms.

unit-pages =
    { $count ->
        [zero] صفحة
        [one] صفحة
        [two] صفحتان
        [few] صفحات
        [many] صفحةً
       *[other] صفحة
    }
unit-rakaat =
    { $count ->
        [zero] ركعة
        [one] ركعة
        [two] ركعتان
        [few] ركعات
        [many] ركعةً
       *[other] ركعة
    }
unit-times =
    { $count ->
        [zero] مرة
        [one] مرة
        [two] مرتان
        [few] مرات
        [many] مرةً
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

## Level sets. Wording from the Academy instructors' sheets (#106), with the
## sheet's group in brackets where the row alone would not say what it is.

set-level-1 = المستوى الأول
set-levels-2-3 = المستويان الثاني والثالث
set-level-advanced = المستوى المتقدم
level-card-1-title = المستوى الأول
level-card-2-title = المستوى الثاني
level-card-3-title = المستوى الثالث
level-card-1-summary = الجماعة الأولى مع السنن، أذكار الصباح والمساء، الورد ربعين وحفظ نصف صفحة، أذكار اليوم، وسنن الجمعة.
level-card-2-summary = يزيد تكبيرة الإحرام، والدعاء بين الأذانين، والضحى، والتهليل، والورد حزبًا، وحفظ صفحة، والدعوة.
level-card-3-summary = التبكير والصف الأول في كل صلاة، والسواك، وقضاء الفوائت، والدعاء للغير، وأعمال القلب واللسان، والمحاسبة.
set-credit-instructors = من أوراق المستويات لمعلمي الأكاديمية، بإذنهم

deed-lv-fajr-early = التبكير (الصبح)
deed-lv-fajr-adhan-adhkar = أذكار الأذان (الصبح)
deed-lv-fajr-dua-between = الدعاء بين الأذانين (الصبح)
deed-lv-fajr-sunnah-before = السنة القبلية (الصبح)
deed-lv-fajr-siwak = السواك (الصبح)
deed-lv-fajr-congregation = الجماعة الأولى (الصبح)
deed-lv-fajr-opening-takbir = تكبيرة الإحرام (الصبح)
deed-lv-fajr-first-row = الصف الأول (الصبح)
deed-lv-fajr-adhkar-after = أذكار بعد الصلاة (الصبح)
deed-lv-fajr-makeup = قضاء الفوائت (الصبح)

deed-lv-morning-adhkar = أذكار الصباح
deed-lv-tahlil-100 = التهليل
deed-lv-morning-tahlil = التهليل (أذكار الصباح)
deed-lv-sitting-until-sunrise = الجلوس إلى الشروق
deed-lv-duha = صلاة الضحى

deed-lv-dhuhr-early = التبكير (الظهر)
deed-lv-dhuhr-adhan-adhkar = أذكار الأذان (الظهر)
deed-lv-dhuhr-dua-between = الدعاء بين الأذانين (الظهر)
deed-lv-dhuhr-sunnah-before-four = السنة القبلية 4 ركعات (الظهر)
deed-lv-dhuhr-sunnah-before = السنة القبلية (الظهر)
deed-lv-dhuhr-siwak = السواك (الظهر)
deed-lv-dhuhr-congregation = الجماعة الأولى (الظهر)
deed-lv-dhuhr-opening-takbir = تكبيرة الإحرام (الظهر)
deed-lv-dhuhr-first-row = الصف الأول (الظهر)
deed-lv-dhuhr-adhkar-after = أذكار بعد الصلاة (الظهر)
deed-lv-dhuhr-sunnah-after = السنة البعدية (الظهر)
deed-lv-dhuhr-makeup = قضاء الفوائت (الظهر)

deed-lv-asr-early = التبكير (العصر)
deed-lv-asr-adhan-adhkar = أذكار الأذان (العصر)
deed-lv-asr-dua-between = الدعاء بين الأذانين (العصر)
deed-lv-asr-sunnah-before = السنة القبلية (العصر)
deed-lv-asr-siwak = السواك (العصر)
deed-lv-asr-congregation = الجماعة الأولى (العصر)
deed-lv-asr-opening-takbir = تكبيرة الإحرام (العصر)
deed-lv-asr-first-row = الصف الأول (العصر)
deed-lv-asr-adhkar-after = أذكار بعد الصلاة (العصر)
deed-lv-asr-makeup = قضاء الفوائت (العصر)
deed-lv-evening-adhkar = أذكار المساء
deed-lv-evening-tahlil = التهليل (أذكار المساء)
deed-lv-sunset-dhikr = ذكر الغروب

deed-lv-maghrib-early = التبكير (المغرب)
deed-lv-maghrib-adhan-adhkar = أذكار الأذان (المغرب)
deed-lv-maghrib-dua-between = الدعاء بين الأذانين (المغرب)
deed-lv-maghrib-sunnah-before = السنة القبلية (المغرب)
deed-lv-maghrib-siwak = السواك (المغرب)
deed-lv-maghrib-congregation = الجماعة الأولى (المغرب)
deed-lv-maghrib-opening-takbir = تكبيرة الإحرام (المغرب)
deed-lv-maghrib-first-row = الصف الأول (المغرب)
deed-lv-maghrib-adhkar-after = أذكار بعد الصلاة (المغرب)
deed-lv-maghrib-sunnah-after = السنة البعدية (المغرب)
deed-lv-maghrib-makeup = قضاء الفوائت (المغرب)

deed-lv-isha-early = التبكير (العشاء)
deed-lv-isha-adhan-adhkar = أذكار الأذان (العشاء)
deed-lv-isha-dua-between = الدعاء بين الأذانين (العشاء)
deed-lv-isha-sunnah-before = السنة القبلية (العشاء)
deed-lv-isha-siwak = السواك (العشاء)
deed-lv-isha-congregation = الجماعة الأولى (العشاء)
deed-lv-isha-opening-takbir = تكبيرة الإحرام (العشاء)
deed-lv-isha-first-row = الصف الأول (العشاء)
deed-lv-isha-adhkar-after = أذكار بعد الصلاة (العشاء)
deed-lv-isha-sunnah-after = السنة البعدية (العشاء)
deed-lv-isha-makeup = قضاء الفوائت (العشاء)

deed-lv-two-rakat-qiyam = ركعتان قيام
deed-lv-wird-two-quarters = الورد (ربعين)
deed-lv-wird-hizb = الورد (حزب)
deed-lv-witr = الوتر
deed-lv-night-rakat = عدد الركعات (الليل)
deed-lv-night-wird = الورد (الليل)
deed-lv-night-istighfar = الاستغفار (الليل)

deed-lv-memorise-half-page = حفظ نصف صفحة
deed-lv-read-six-quarters = قراءة ستة أرباع
deed-lv-memorise-page = حفظ صفحة
deed-lv-read-juz = قراءة جزء
deed-lv-recite-reflect = تلاوة وتدبر
deed-lv-review-memorise = مراجعة وحفظ

deed-lv-dua-self = الدعاء للنفس
deed-lv-dua-teachers = الدعاء للمشايخ
deed-lv-dua-brothers = الدعاء للإخوة
deed-lv-dua-muslims = الدعاء للمسلمين

deed-lv-fasting-monday-thursday = الصيام (الاثنين والخميس)
deed-lv-fasting = الصيام

deed-lv-waking-adhkar = ذكر الاستيقاظ
deed-lv-toilet-adhkar = ذكر الخلاء
deed-lv-dressing-adhkar = ذكر لبس الثوب وخلعه
deed-lv-wudu-adhkar = ذكر الوضوء
deed-lv-home-adhkar = ذكر دخول المنزل والخروج
deed-lv-mosque-adhkar = ذكر المسجد (دخول وخروج)
deed-lv-mosque-enter-leave-adhkar = ذكر دخول المسجد والخروج
deed-lv-walking-to-mosque-adhkar = ذكر المشي إلى المسجد
deed-lv-food-adhkar = ذكر الأكل والشرب
deed-lv-riding-adhkar = ذكر الركوب
deed-lv-sleep-adhkar = ذكر النوم
deed-lv-istighfar-100 = الاستغفار
deed-lv-gathering-istighfar = الاستغفار في المجالس
deed-lv-gathering-kaffara = كفارة المجالس
deed-lv-general-istighfar = الاستغفار المطلق
deed-lv-salawat = الصلاة على النبي ﷺ

deed-lv-attend-lessons = حضور دروس العلم (السبت والخميس)
deed-lv-review-lessons = مذاكرة دروس العلم
deed-lv-parents = بر الوالدين
deed-lv-study-or-work = مذاكرة الدراسة أو إتقان العمل الدنيوي (خمس ساعات)
deed-lv-dawah = الدعوة
deed-lv-kinship = صلة الرحم
deed-lv-rights = أداء الحقوق
deed-lv-tongue = ترك آفات اللسان
deed-lv-gaze = ترك فضول النظر
deed-lv-heart = معالجة القلب
deed-lv-charity = الصدقة
deed-lv-istikhara = صلاة الاستخارة
deed-lv-hajah = صلاة الحاجة
deed-lv-keep-wudu = المحافظة على الوضوء
deed-lv-wudu-before-sleep = الوضوء للنوم
deed-lv-worldly-knowledge = العلم الدنيوي
deed-lv-attend-sharia = حضور العلم الشرعي
deed-lv-review-sharia = مذاكرة العلم الشرعي
deed-lv-self-review = المحاسبة
deed-lv-repentance = التوبة

deed-lv-friday-fitra = سنن الفطرة (الجمعة)
deed-lv-friday-ghusl = الغسل (الجمعة)
deed-lv-friday-best-clothes = لبس أفضل الثياب (الجمعة)
deed-lv-friday-perfume = الطيب (الجمعة)
deed-lv-friday-siwak = السواك (الجمعة)
deed-lv-friday-best-mosque = اختيار المسجد الأفضل (الجمعة)
deed-lv-friday-early = التبكير (الجمعة)
deed-lv-friday-dua-imam = الدعاء عند صعود الإمام (الجمعة)
deed-lv-friday-dua-before-maghrib = الدعاء قبل المغرب (الجمعة)
deed-lv-friday-dua-after-asr = الدعاء بعد العصر (الجمعة)
deed-lv-friday-kahf = سورة الكهف (الجمعة)
deed-lv-friday-salawat-100 = الصلاة على النبي (الجمعة)
deed-lv-friday-salawat = الصلاة على النبي ﷺ (الجمعة)
