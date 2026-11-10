# Starter set names, deed names and target units (ADR 0019, ADR 0025).
# Keys come from data/starter-sets.toml. Our own wording, except the level sets at the end.

## Units. $count is the target amount.

unit-pages =
    { $count ->
        [one] page
       *[other] pages
    }
unit-rakaat =
    { $count ->
        [one] rakah
       *[other] rakaat
    }
unit-times =
    { $count ->
        [one] time
       *[other] times
    }

## Sets.

set-five-prayers = The five prayers
set-sunnah-prayers = Sunnah prayers
set-mosque-prayer = Prayer in the mosque
set-with-every-prayer = With every prayer
set-morning-evening = Morning and evening
set-night-prayer = Night prayer
set-quran = Quran
set-daily-adhkar = Everyday adhkar
set-friday = Friday
set-knowledge-dawah = Knowledge and dawah
set-fasting-seasons = Fasting and seasons
set-ramadan = Ramadan

## The five prayers.

deed-fajr-on-time = Fajr on time
deed-dhuhr-on-time = Dhuhr on time
deed-asr-on-time = Asr on time
deed-maghrib-on-time = Maghrib on time
deed-isha-on-time = Isha on time

## Sunnah prayers.

deed-sunnah-before-fajr = Sunnah before Fajr
deed-sunnah-before-dhuhr = Sunnah before Dhuhr
deed-sunnah-after-dhuhr = Sunnah after Dhuhr
deed-sunnah-after-maghrib = Sunnah after Maghrib
deed-sunnah-after-isha = Sunnah after Isha

## Prayer in the mosque.

deed-fajr-congregation = Fajr in congregation from the start
deed-fajr-opening-takbir = Opening takbir with the imam at Fajr
deed-fajr-first-row = First row at Fajr
deed-dhuhr-congregation = Dhuhr in congregation from the start
deed-dhuhr-opening-takbir = Opening takbir with the imam at Dhuhr
deed-dhuhr-first-row = First row at Dhuhr
deed-asr-congregation = Asr in congregation from the start
deed-asr-opening-takbir = Opening takbir with the imam at Asr
deed-asr-first-row = First row at Asr
deed-maghrib-congregation = Maghrib in congregation from the start
deed-maghrib-opening-takbir = Opening takbir with the imam at Maghrib
deed-maghrib-first-row = First row at Maghrib
deed-isha-congregation = Isha in congregation from the start
deed-isha-opening-takbir = Opening takbir with the imam at Isha
deed-isha-first-row = First row at Isha

## With every prayer.

deed-fajr-dua-before-iqama = Dua before the iqama of Fajr
deed-fajr-adhkar-after = Adhkar after Fajr
deed-dhuhr-dua-before-iqama = Dua before the iqama of Dhuhr
deed-dhuhr-adhkar-after = Adhkar after Dhuhr
deed-asr-dua-before-iqama = Dua before the iqama of Asr
deed-asr-adhkar-after = Adhkar after Asr
deed-maghrib-dua-before-iqama = Dua before the iqama of Maghrib
deed-maghrib-adhkar-after = Adhkar after Maghrib
deed-isha-dua-before-iqama = Dua before the iqama of Isha
deed-isha-adhkar-after = Adhkar after Isha

## Morning and evening.

deed-morning-adhkar = Morning adhkar
deed-tahlil = La ilaha illa Allah
deed-sitting-until-sunrise = Sitting until sunrise
deed-duha = Duha prayer
deed-evening-adhkar = Evening adhkar
deed-istighfar = Istighfar
deed-sleep-adhkar = Adhkar before sleep

## Night prayer.

deed-qiyam = Qiyam
deed-witr = Witr

## Quran.

deed-quran-reading = Quran reading
deed-quran-memorising = New memorisation
deed-quran-review = Review what I memorised

## Everyday adhkar.

deed-waking-adhkar = On waking up
deed-toilet-adhkar = Entering and leaving the toilet
deed-wudu-adhkar = Wudu
deed-dressing-adhkar = Getting dressed
deed-home-adhkar = Entering and leaving home
deed-food-adhkar = Eating and drinking
deed-riding-adhkar = Riding
deed-mosque-adhkar = Entering and leaving the mosque
deed-walking-to-mosque-adhkar = Walking to the mosque

## Friday.

deed-friday-ghusl = Friday ghusl
deed-sunan-al-fitra = Sunan al fitra
deed-best-clothes-and-perfume = Best clothes and perfume
deed-siwak = Siwak
deed-early-to-jumuah = Going early to Jumuah
deed-surah-al-kahf = Surah al Kahf
deed-salawat = Salawat on the Prophet
deed-friday-last-hour-dua = Dua in the last hour of Friday

## Knowledge and dawah.

deed-attending-lesson = Attending a lesson
deed-reviewing-lessons = Reviewing lessons
deed-dawah = Dawah

## Fasting and seasons.

deed-fasting-monday-thursday = Fasting Monday and Thursday
deed-fasting-ayyam-al-bid = Fasting the white days
deed-fasting-ashura = Fasting Tasua and Ashura
deed-charity-ashura = Charity on Ashura
deed-family-ashura = Generosity to family on Ashura
deed-fasting-dhul-hijjah = Fasting the first nine of Dhul Hijjah
deed-fasting-arafah = Fasting the Day of Arafah
deed-takbir-dhul-hijjah = Takbir in the days of Dhul Hijjah

## Ramadan.

deed-ramadan-fasting = Fasting Ramadan
deed-taraweeh = Taraweeh
deed-ramadan-quran = Quran in Ramadan
deed-ramadan-charity = Charity in Ramadan
deed-last-ten-qiyam = Qiyam in the last ten nights
deed-laylat-al-qadr-dua = Dua for Laylat al Qadr

## Level sets. Glosses of the Academy instructors' sheets (#106), with the
## sheet's group in brackets where the row alone would not say what it is.

set-level-1 = Level 1
set-levels-2-3 = Levels 2 and 3
set-level-advanced = Advanced level
level-card-1-title = Academy level 1
level-card-2-title = Academy level 2
level-card-3-title = Academy level 3
level-card-1-summary = The first congregation with its sunnah, morning and evening adhkar, two quarters a day and half a page to memorise, the adhkar of the day, Friday sunnahs.
level-card-2-summary = Adds the opening takbir, dua between adhan and iqamah, duha, tahlil, a hizb a day, a page to memorise, dawah.
level-card-3-summary = Going early and the first row at every prayer, siwak, making up missed prayers, dua for others, deeds of the heart and tongue, self review.
set-credit-instructors = From the Academy instructors' level sheets, used with their permission

deed-lv-fajr-early = Going early (Fajr)
deed-lv-fajr-adhan-adhkar = Adhkar of the adhan (Fajr)
deed-lv-fajr-dua-between = Dua between adhan and iqamah (Fajr)
deed-lv-fajr-sunnah-before = Sunnah before (Fajr)
deed-lv-fajr-siwak = Siwak (Fajr)
deed-lv-fajr-congregation = First congregation (Fajr)
deed-lv-fajr-opening-takbir = Opening takbir (Fajr)
deed-lv-fajr-first-row = First row (Fajr)
deed-lv-fajr-adhkar-after = Adhkar after the prayer (Fajr)
deed-lv-fajr-makeup = Making up missed prayers (Fajr)

deed-lv-morning-adhkar = Morning adhkar
deed-lv-tahlil-100 = Tahlil
deed-lv-morning-tahlil = Tahlil (morning adhkar)
deed-lv-sitting-until-sunrise = Sitting until sunrise
deed-lv-duha = Duha prayer

deed-lv-dhuhr-early = Going early (Dhuhr)
deed-lv-dhuhr-adhan-adhkar = Adhkar of the adhan (Dhuhr)
deed-lv-dhuhr-dua-between = Dua between adhan and iqamah (Dhuhr)
deed-lv-dhuhr-sunnah-before-four = Sunnah before, 4 rakat (Dhuhr)
deed-lv-dhuhr-sunnah-before = Sunnah before (Dhuhr)
deed-lv-dhuhr-siwak = Siwak (Dhuhr)
deed-lv-dhuhr-congregation = First congregation (Dhuhr)
deed-lv-dhuhr-opening-takbir = Opening takbir (Dhuhr)
deed-lv-dhuhr-first-row = First row (Dhuhr)
deed-lv-dhuhr-adhkar-after = Adhkar after the prayer (Dhuhr)
deed-lv-dhuhr-sunnah-after = Sunnah after (Dhuhr)
deed-lv-dhuhr-makeup = Making up missed prayers (Dhuhr)

deed-lv-asr-early = Going early (Asr)
deed-lv-asr-adhan-adhkar = Adhkar of the adhan (Asr)
deed-lv-asr-dua-between = Dua between adhan and iqamah (Asr)
deed-lv-asr-sunnah-before = Sunnah before (Asr)
deed-lv-asr-siwak = Siwak (Asr)
deed-lv-asr-congregation = First congregation (Asr)
deed-lv-asr-opening-takbir = Opening takbir (Asr)
deed-lv-asr-first-row = First row (Asr)
deed-lv-asr-adhkar-after = Adhkar after the prayer (Asr)
deed-lv-asr-makeup = Making up missed prayers (Asr)
deed-lv-evening-adhkar = Evening adhkar
deed-lv-evening-tahlil = Tahlil (evening adhkar)
deed-lv-sunset-dhikr = Dhikr at sunset

deed-lv-maghrib-early = Going early (Maghrib)
deed-lv-maghrib-adhan-adhkar = Adhkar of the adhan (Maghrib)
deed-lv-maghrib-dua-between = Dua between adhan and iqamah (Maghrib)
deed-lv-maghrib-sunnah-before = Sunnah before (Maghrib)
deed-lv-maghrib-siwak = Siwak (Maghrib)
deed-lv-maghrib-congregation = First congregation (Maghrib)
deed-lv-maghrib-opening-takbir = Opening takbir (Maghrib)
deed-lv-maghrib-first-row = First row (Maghrib)
deed-lv-maghrib-adhkar-after = Adhkar after the prayer (Maghrib)
deed-lv-maghrib-sunnah-after = Sunnah after (Maghrib)
deed-lv-maghrib-makeup = Making up missed prayers (Maghrib)

deed-lv-isha-early = Going early (Isha)
deed-lv-isha-adhan-adhkar = Adhkar of the adhan (Isha)
deed-lv-isha-dua-between = Dua between adhan and iqamah (Isha)
deed-lv-isha-sunnah-before = Sunnah before (Isha)
deed-lv-isha-siwak = Siwak (Isha)
deed-lv-isha-congregation = First congregation (Isha)
deed-lv-isha-opening-takbir = Opening takbir (Isha)
deed-lv-isha-first-row = First row (Isha)
deed-lv-isha-adhkar-after = Adhkar after the prayer (Isha)
deed-lv-isha-sunnah-after = Sunnah after (Isha)
deed-lv-isha-makeup = Making up missed prayers (Isha)

deed-lv-two-rakat-qiyam = Two rakat of night prayer
deed-lv-wird-two-quarters = Daily portion (two quarters)
deed-lv-wird-hizb = Daily portion (one hizb)
deed-lv-witr = Witr
deed-lv-night-rakat = Number of rakat (night)
deed-lv-night-wird = Daily portion (night)
deed-lv-night-istighfar = Istighfar (night)

deed-lv-memorise-half-page = Memorise half a page
deed-lv-read-six-quarters = Read six quarters
deed-lv-memorise-page = Memorise one page
deed-lv-read-juz = Read one juz
deed-lv-recite-reflect = Recitation and reflection
deed-lv-review-memorise = Revision and memorisation

deed-lv-dua-self = Dua for yourself
deed-lv-dua-teachers = Dua for the teachers
deed-lv-dua-brothers = Dua for the brothers
deed-lv-dua-muslims = Dua for all Muslims

deed-lv-fasting-monday-thursday = Fasting (Monday and Thursday)
deed-lv-fasting = Fasting

deed-lv-waking-adhkar = Dhikr on waking
deed-lv-toilet-adhkar = Dhikr for the toilet
deed-lv-dressing-adhkar = Dhikr for dressing and undressing
deed-lv-wudu-adhkar = Dhikr for wudu
deed-lv-home-adhkar = Dhikr for entering and leaving home
deed-lv-mosque-adhkar = Dhikr for the mosque (entering and leaving)
deed-lv-mosque-enter-leave-adhkar = Dhikr for entering and leaving the mosque
deed-lv-walking-to-mosque-adhkar = Dhikr walking to the mosque
deed-lv-food-adhkar = Dhikr for eating and drinking
deed-lv-riding-adhkar = Dhikr for riding
deed-lv-sleep-adhkar = Dhikr before sleep
deed-lv-istighfar-100 = Istighfar
deed-lv-gathering-istighfar = Istighfar in gatherings
deed-lv-gathering-kaffara = Dua at the close of a gathering
deed-lv-general-istighfar = General istighfar
deed-lv-salawat = Salawat on the Prophet

deed-lv-attend-lessons = Attending knowledge lessons (Saturday and Thursday)
deed-lv-review-lessons = Reviewing the knowledge lessons
deed-lv-parents = Kindness to parents
deed-lv-study-or-work = Study or doing worldly work well (five hours)
deed-lv-dawah = Dawah
deed-lv-kinship = Keeping family ties
deed-lv-rights = Giving people their rights
deed-lv-tongue = Avoiding the faults of the tongue
deed-lv-gaze = Lowering the gaze
deed-lv-heart = Treating the heart
deed-lv-charity = Charity
deed-lv-istikhara = Istikhara prayer
deed-lv-hajah = Salat al hajah (prayer of need)
deed-lv-keep-wudu = Staying in wudu
deed-lv-wudu-before-sleep = Wudu before sleep
deed-lv-worldly-knowledge = Worldly knowledge
deed-lv-attend-sharia = Attending lessons in sacred knowledge
deed-lv-review-sharia = Reviewing sacred knowledge
deed-lv-self-review = Self review
deed-lv-repentance = Repentance

deed-lv-friday-fitra = Sunan al fitra (Friday)
deed-lv-friday-ghusl = Ghusl (Friday)
deed-lv-friday-best-clothes = Wearing your best clothes (Friday)
deed-lv-friday-perfume = Perfume (Friday)
deed-lv-friday-siwak = Siwak (Friday)
deed-lv-friday-best-mosque = Choosing the best mosque (Friday)
deed-lv-friday-early = Going early to Jumuah
deed-lv-friday-dua-imam = Dua when the imam ascends the minbar
deed-lv-friday-dua-before-maghrib = Dua before Maghrib (Friday)
deed-lv-friday-dua-after-asr = Dua after Asr (Friday)
deed-lv-friday-kahf = Surat al Kahf (Friday)
deed-lv-friday-salawat-100 = Salawat on the Prophet (Friday)
deed-lv-friday-salawat = Salawat on the Prophet (Friday)
