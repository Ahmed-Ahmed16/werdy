# Starter set names, deed names and target units (ADR 0019, ADR 0025).
# Keys come from data/starter-sets.toml. Our own wording.

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
