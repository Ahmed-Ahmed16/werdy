# Starter sets: first draft (ADR 0019)

Draft for Ahmed's review, 2026-10-27. Starter sets are data (ADR 0024); this file is the content, not the format.

## Sources and the clean room line

* Ahmed's own tracker sheet, `جدول المحاسبة 1.pdf` (a weekly muhasaba table: deeds by prayer, a points column, Saturday to Friday columns, a daily percentage). Used for which kinds of deeds a serious owner tracks and how fine grained they go.
* `وظائف محرم وعاشوراء.pdf`, a study handout from the academy. Used only for which seasonal deeds exist (fasting in Muharram, fasting the 9th and 10th, charity, generosity to family on Ashura). Its text, wording and layout are not copied (ADR 0001).
* Common practice. Every deed name below is our own short wording.
* Werd's presets were not opened. Werd may remind us which kinds of deeds people expect, but the PDFs already cover that, so there was no need.
* Points from the sheet are dropped: every deed weighs the same (ADR 0008).

Answered on 2026-10-27: `جدول المحاسبة` is Ahmed's own sheet, adapted with his own edits from the Werd program he uses on his account. So it is close to Werd's program, and the clean room line holds even more firmly: kinds of deeds only, our own wording, our own order, no points.

## What the sheet teaches us

1. Serious owners track each prayer in fine parts (first takbir, first row, adhkar after). Beginners would drown in that. So sets come in layers: start small, add a layer when ready.
2. The sheet already counts Friday deeds only on Friday (its daily total changes on Friday). That is exactly scoring rule 1.
3. Its week runs Saturday to Friday. werdy's periods are rolling (ADR 0012), so this does not affect scores; the week start for any calendar view is a setting, default by country, `___`.
4. Ashura (9 and 10 Muharram) and the Day of Arafah are not in ADR 0009's list of Hijri ranges. Ranges are data (ADR 0024), so adding them needs no code change, only the range entries.

## Sets

Columns: deed, time slot, category, schedule, target.

### 1. The five prayers (basic, the default for first run)
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Fajr on time | Fajr | Prayer | Every day | none |
| Dhuhr on time | Dhuhr | Prayer | Every day | none |
| Asr on time | Asr | Prayer | Every day | none |
| Maghrib on time | Maghrib | Prayer | Every day | none |
| Isha on time | Isha | Prayer | Every day | none |

### 2. Sunnah prayers with the fard
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Sunnah before Fajr | Fajr | Prayer | Every day | none |
| Sunnah before Dhuhr | Dhuhr | Prayer | Every day | 4 rakaat |
| Sunnah after Dhuhr | Dhuhr | Prayer | Every day | none |
| Sunnah after Maghrib | Maghrib | Prayer | Every day | none |
| Sunnah after Isha | Isha | Prayer | Every day | none |

### 3. Prayer in the mosque (one row per prayer, so 15 deeds)
For each of Fajr, Dhuhr, Asr, Maghrib, Isha, in that slot, category Prayer, every day:
* In congregation, from its start
* The opening takbir with the imam
* In the first row

### 4. Around the prayer (10 deeds)
For each of the five prayers, in its slot, category Adhkar, every day:
* Dua between adhan and iqama
* Adhkar after the prayer

### 5. Morning and evening
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Morning adhkar | Morning | Adhkar | Every day | none |
| La ilaha illa Allah | Morning | Adhkar | Every day | 100 times |
| Sitting until sunrise | Morning | Adhkar | Every day | none |
| Duha prayer | Morning | Prayer | Every day | none |
| Evening adhkar | Asr | Adhkar | Every day | none |
| Istighfar | Any time | Adhkar | Every day | 100 times |
| Adhkar before sleep | Night | Adhkar | Every day | none |

### 6. Night prayer
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Qiyam | Night | Prayer | Every day | 4 rakaat |
| Witr | Night | Prayer | Every day | none |

### 7. Quran
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Reading | Any time | Quran | Every day | 20 pages (one juz) |
| New memorisation | Any time | Quran | Every day | none (a quarter hizb) |
| Review of memorised parts | Any time | Quran | Every day | none |

### 8. Daily adhkar of life (optional, 10 deeds)
Any time, category Adhkar, every day: on waking, entering and leaving the toilet, wudu, dressing, entering and leaving home, eating and drinking, riding, entering and leaving the mosque, walking to the mosque, before sleep is already in set 5.

### 9. Friday
Schedule: Fridays. Category in brackets.
| Deed | Slot |
| --- | --- |
| Ghusl (Other) | Morning |
| Sunan al fitra (Other) | Morning |
| Best clothes and perfume (Other) | Morning |
| Siwak (Other) | Any time |
| Going early to Jumuah (Prayer) | Dhuhr |
| Surah al Kahf (Quran) | Any time |
| Salawat on the Prophet, 100 times (Adhkar) | Any time |
| Dua in the last hour before Maghrib (Adhkar) | Asr |

### 10. Knowledge and dawah
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Attending a lesson | Any time | Study | Every day (the owner narrows to weekdays) | none |
| Reviewing lessons | Any time | Study | Every day | none |
| Dawah | Any time | Other | Every day | none |

### 11. Seasonal fasting (each deed has its own Hijri or weekday schedule)
| Deed | Slot | Category | Schedule |
| --- | --- | --- | --- |
| Fasting Monday and Thursday | Any time | Fasting | Weekdays: Monday, Thursday |
| Fasting Ayyam al Bid | Any time | Fasting | Hijri: 13 to 15 of each month |
| Fasting Ashura and the day before | Any time | Fasting | Hijri: 9 and 10 Muharram |
| Charity on Ashura | Any time | Charity | Hijri: 10 Muharram |
| Generosity to family on Ashura | Any time | Family | Hijri: 10 Muharram |
| Fasting the first nine of Dhul Hijjah | Any time | Fasting | Hijri: 1 to 9 Dhul Hijjah |
| Fasting the Day of Arafah | Any time | Fasting | Hijri: 9 Dhul Hijjah |
| Takbir in the ten days | Any time | Adhkar | Hijri: 1 to 13 Dhul Hijjah, `___` (end day to confirm) |

### 12. Ramadan (proposed from common practice, Ahmed to confirm)
| Deed | Slot | Category | Schedule | Target |
| --- | --- | --- | --- | --- |
| Fasting | Any time | Fasting | Hijri: Ramadan | none |
| Taraweeh | Isha | Prayer | Hijri: Ramadan | none |
| Quran in Ramadan | Any time | Quran | Hijri: Ramadan | `___` pages |
| Charity in Ramadan | Any time | Charity | Hijri: Ramadan | none |
| Qiyam in the last ten nights | Night | Prayer | Hijri: last ten nights | none |
| Dua for Laylat al Qadr | Night | Adhkar | Hijri: last ten nights | none |

## Notes for the data format

* One deed per row; a set is a named list of deeds. A test checks every set uses only known slots, categories and Hijri ranges.
* Names are Fluent keys with Arabic and English text, so a set can be reworded without code.
* Overlap: set 5 and set 8 both touch sleep adhkar; adding both sets must not create a duplicate deed. Rule: same key, added once.
* "Sitting until sunrise" and Duha may suit only some owners; sets are a starting point, every deed can be removed after one tap.
