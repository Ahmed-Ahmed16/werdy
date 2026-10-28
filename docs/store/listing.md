# Play Store listing drafts

Draft for #71. Copy only the field text into the store. Arabic first.
Owner review and release checks remain pending.

## العربية

### اسم التطبيق

werdy

### وصف قصير

تابع عباداتك وأعمالك اليومية، وراجع تقدمك في الأيام المستحقة فقط.

### وصف كامل

werdy سجل لأعمالك اليومية من عبادة ودراسة وأسرة وصحة وعمل. اختر مجموعة جاهزة للصلاة أو القرآن أو الأذكار، أو أضف أعمالك بنفسك.

رتب أعمالك حسب وقتها في اليوم، وحدد موعد استحقاق كل عمل: كل يوم، أو أيامًا معينة من الأسبوع، أو فترة هجرية. يمكنك تحديد هدف بعدد، مثل صفحات القرآن.

سجل ما فعلته بعلامة تم أو جزئي أو فات. وللعمل ذي الهدف العددي، سجل مقدار ما أنجزت. يمكنك إضافة ملاحظة لليوم، واختيار احتساب العذر بدون زيادة أو بنصف أو بكامل قيمة الأعمال المستحقة.

راجع تقارير 3 أيام أو أسبوع أو 30 يومًا أو 6 أشهر أو سنة. ترى النتائج حسب الفئة وحسب كل عمل، وما فاتك خلال الفترة. تحسب النتائج الأيام التي كان العمل مستحقًا فيها فقط. يبقى اليوم الحالي قيد التقدم، وتغطي التقارير الأيام المنتهية حتى أمس.

اختر مدينة أو موقعك لحساب بداية اليوم عند الفجر على جهازك. يمكنك المتابعة دون اختيار مكان. مكانك وطريقة حساب الصلاة يبقيان على الجهاز ولا يرسلهما التطبيق إلى خادم المزامنة أو إلى من تشارك معه.

استخدم التطبيق دون حساب، وسجل أعمالك دون اتصال بالإنترنت. يمكنك تسجيل الدخول باستخدام Google لمزامنة سجلك بين أجهزتك. ويمكنك مشاركة صفحتك وتقاريرك ونص ملاحظاتك مع شخص تختاره للقراءة فقط، ثم إلغاء وصوله من الإعدادات. تحتاج المزامنة والمشاركة إلى اتصال بالإنترنت وحساب.

الواجهة بالعربية والإنجليزية، مع مظهر فاتح أو داكن أو حسب إعداد الجهاز.

## English

### App name

werdy

### Short description

Track daily deeds and review progress on the days each deed was due.

### Full description

werdy keeps a daily record of worship, study, family, health and work. Choose starter sets for prayer, Quran or adhkar, or add your own deeds.

Group deeds by time of day. Make each deed due every day, on chosen weekdays or during a Hijri date range. Add a count target, such as pages of Quran.

Mark deeds Done, Partial or Missed. For a deed with a count target, record how much you did. Add a day note and choose no excuse credit, half credit or full credit for that day's due deeds.

Review reports for 3 days, a week, 30 days, 6 months or a year. See results by category and deed, and see what you missed. Scores count only the days each deed was due. Today stays in progress. Reports cover finished days up to yesterday.

Choose a city or your location to calculate the start of your day at Fajr on your device. You can continue without choosing a place. Your saved location and prayer calculation method stay on the device. The app does not send them to the sync server or your watcher.

Use werdy without an account and record deeds without an internet connection. You can sign in with Google to sync records across your devices. You can also let someone you choose read your sheet, reports and note text, then revoke their access in Settings. Sync and sharing need an internet connection and an account.

The interface supports Arabic and English, with light, dark and system themes.

## Repo evidence

Field lengths include spaces and punctuation, excluding headings and surrounding
blank lines. Neither draft contains supplementary Unicode characters.

| Language | App name | Short description, limit 80 | Full description, limit 4000 |
| --- | --- | --- | --- |
| Arabic | 5 | 65 | 1232 |
| English | 5 | 68 | 1391 |

* Name: `locales/ar/common.ftl`, `locales/en/common.ftl` and `crates/werdy-app/Dioxus.toml`.
* Deeds, schedules, targets and starter sets: `CONTEXT.md`, `locales/ar/deeds.ftl`, `locales/en/deeds.ftl` and `data/starter-sets.toml`.
* Marks, notes and reports: `locales/ar/sheet.ftl`, `locales/en/sheet.ftl`, `locales/ar/reports.ftl`, `locales/en/reports.ftl` and ADR 0028 in `docs/decisions.md`.
* Location, account and sharing: ADRs 0010, 0015 and 0039, `docs/context/account-sync.md`, `docs/context/sharing.md` and `crates/werdy-sync/src/rows.rs`.
* Language and themes: `locales/ar/settings.ftl` and `locales/en/settings.ftl`.

The account purpose text in both settings locales still says sharing comes later.
Sharing is wired in `crates/werdy-app/src/ui/settings.rs` and documented in
`docs/context/sharing.md`. This draft follows that implementation. Owner must
verify the shipping build and resolve that stale UI text separately.
