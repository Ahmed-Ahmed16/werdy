# UX research for werdy v1

Gathered on 2026-10-27 to guide the v1 screen design. Each point says how strong its source is. Points marked **weak** come from vendor blogs or secondary summaries and are hints, not facts.

## 1. General heuristics (Nielsen's ten, applied)

These are well established and need no new evidence here. The ones that matter most for werdy:

* **Visibility of system status:** the owner always sees where today stands (done, partial, open) without opening a report.
* **Match with the real world:** the sheet follows the day (Fajr to night), not an abstract list of sections.
* **User control and freedom:** every mark can be undone with one more tap; deleting a deed asks first and archiving is the default.
* **Consistency:** a mark looks the same on the sheet, in the strip and in reports.
* **Error prevention:** no date pickers for reports (swipe periods instead); deed changes never rewrite the past (ADR 0020).
* **Recognition over recall:** starter sets and category chips instead of empty forms.
* **Minimalist design:** only deeds due today are shown; the rest are one line at the bottom.

## 2. Touch targets

* WCAG 2.2 success criterion 2.5.8 (level AA) sets a floor of 24 by 24 CSS px. Criterion 2.5.5 (AAA) asks for 44 by 44. **Strong** (W3C standard, as summarised in [disabilityworld.org 2.5.8](https://www.disabilityworld.org/toolkit/standards/wcag/2-5-8-target-size-minimum/)).
* Apple recommends 44 by 44 pt; Android recommends at least 48 by 48 dp. **Strong** (platform guidelines, summarised in [Smashing Magazine](https://shop.smashingmagazine.com/2024/07/getting-bottom-minimum-wcag-conformant-interactive-element-size/)).
* **For werdy:** every mark button and stepper is 48 by 48 or larger, with at least 6 px between neighbours. Marking happens one handed, often right after prayer.

## 3. Colour and marks

* WCAG 1.4.1 (level A): colour may carry meaning but must never be the only signal. **Strong** ([MDN, Use of color](https://developer.mozilla.org/docs/Web/Accessibility/Understanding_WCAG/Perceivable/Use_of_color)).
* Red and green alone are hard for roughly 1 in 12 men. Pair each status with a distinct shape and a text label; NHS Scotland's status palette does this with shapes. **Strong** ([NHS Scotland status palette](https://www.turasdesignsystem.nhs.scot/design/data-visualisation/status-palette/)).
* **For werdy:** Done is a dark green with a check, Partial is a light amber with a half filled circle, Missed is red with a cross. They also differ in lightness, so they still read in greyscale. Today, still in progress, is an outlined neutral cell, never a colour.

## 4. Arabic and right to left

* Mirror the layout and directional icons (back, forward, chevrons). Do not mirror numbers, checkmarks, clocks, or media controls. **Strong** ([Material Design bidirectionality](https://m2.material.io/go/design-bidirectionality), [Mozilla RTL guidelines](https://firefox-source-docs.mozilla.org/code-quality/coding-style/rtl_guidelines.html)).
* **For werdy:** "previous week" points right in Arabic and left in English. Percentages and counts stay as written. Western digits by default (ADR 0014).

## 5. Loading and speed

* Evidence on skeleton screens versus spinners is mixed: one study (Viget, 2017) found spinners felt faster. **Weak** (reported second hand in [freeCodeCamp](https://www.freecodecamp.org/news/how-to-use-skeleton-screens-to-improve-perceived-website-performance/) and [onething.design](https://www.onething.design/post/skeleton-screens-vs-loading-spinners)).
* **For werdy:** the better answer is to avoid the wait altogether. Data is local first (ADR 0015), so the sheet and reports render from the phone at once. Sync runs in the background and never blocks a screen. No number is ever drawn as 0% before the real value (a Werd bug).

## 6. Self tracking, missed days and streaks

* People move through stages when they track themselves (preparation, collection, integration, reflection, action), and a barrier at one stage carries into later ones. **Strong** (Li, Dey and Forlizzi, CHI 2010, [summary](https://quantifiedself.com/blog/a-stage-based-model-of-personal-informatics-systems/)).
* People lapse and resume tracking as part of normal life; design should make coming back easy, not punish the gap. **Strong** (Epstein et al., CHI 2015, "A lived informatics model of personal informatics", [summary](https://library.selfresearch.org/research/a-lived-informatics-model-of-personal-informatics/)).
* A single failure can trigger "what the hell, I already broke it" and then abandonment (Polivy and Herman, dieting research). Streak resets feel like a total loss rather than a proportional one. **Weak** for apps specifically (vendor blogs such as [habitdoom.com](https://habitdoom.com/blog/streak-anxiety-habit-trackers) and [goalsandprogress.com](https://goalsandprogress.com/two-day-rule-habit-recovery/)); the claim that Lally et al. (2010) found one missed day does not hurt habit formation should be checked against the paper, `___`.
* **For werdy:** this supports ADR 0012 (streaks parked). If streaks come, prefer a forgiving form, such as "done on 5 of the last 7 due days", over an all or nothing counter. Late marks stay easy (ADR 0021), and a lapsed owner opens the app to today, not to a wall of red.

## 7. Werd's own UI problems, mapped

| Werd problem (from `werd-behaviour.md`) | werdy answer |
| --- | --- |
| Seconds of spinner on start and on tab switch | Local data, no blocking load (section 5) |
| Date picker OK needs a second tap | No date pickers; swipe or arrow between periods |
| Side menu clipped at some sizes | No side menu; three bottom tabs |
| Report ring shows 0% before the real value | Numbers render from local data on first paint |
| One long sheet of 15 sections | Only due deeds, grouped by time slot, empty slots hidden |
| Checkboxes too small to hit with a finger (owner, as a daily user) | Mark buttons 48 by 48 or larger (section 2) |
| Too much scrolling to find the deed just done (owner) | Fully done time slots fold into one line; the sheet opens at the current slot |
| Hard to mark a past day after forgetting (owner) | Arrows beside the date step to yesterday and earlier; late marks are flagged (ADR 0021) |
| Reports hard to read; they do not say what to fix (owner) | Each report leads with one sentence and the "what did I miss" list, numbers second |
| Editing deeds is clumsy (owner) | One screen editor with chips, sensible defaults, and a plain sentence that reads the schedule back |

## 8. Second pass (2026-10-27): tablets, the web, long ranges, Arabic type, first run

Gathered by a fast research agent; its full notes stayed outside the repo. Only the findings werdy acts on are kept here.

* Android window width classes: Compact below 600dp, Medium 600 to 840dp, Expanded 840dp and up. Design from width alone. **Strong** ([Android, window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/window-size-classes)). **For werdy:** phone layout below 600, tablet two pane layout from 840, and the same breakpoints on the web (ADR 0022).
* Google's large screen quality tiers: Tier 3 means full screen in every orientation and in split screen, never letterboxed; Tier 2 adds adaptive layouts, keyboard navigation, shortcuts and hover. **Strong** ([Android, large screen app quality](https://developer.android.com/docs/quality-guidelines/large-screens-app-quality)). **For werdy:** aim for Tier 2 in v1. Every mark button gets a visible focus ring and a keyboard path; landscape and split screen are part of every release check.
* Navigation: bottom bar on phones, navigation rail on tablets; no drawer while there are only three destinations. **Weak** (drawer guidance may be changing). **For werdy:** matches the phone and tablet frames.
* PWA install needs HTTPS, a manifest with 192 and 512 px icons, and a user interaction first; the app must work without being installed. **Strong** ([web.dev, install criteria](https://web.dev/install-criteria/)).
* Long ranges: bar charts are the common, safe default; calendar heatmaps show missed days well but no study compares the two for habit tracking. **Weak** ([arXiv 2202.10620](https://arxiv.org/pdf/2202.10620)). **For werdy:** bars for 6 months, a calendar for the month, and a small user test before the year view is final. Every rate shows its "out of N due".
* Arabic body text: about 1.5 line height, and check that tashkeel does not clip. **Weak** ([Adobe Spectrum](https://spectrum.adobe.com/page/body)). Body text floor of 16 px in both scripts. The design uses IBM Plex Sans Arabic; Noto Sans Arabic is the alternative to compare on real deed names, `___`.
* First run: an empty state says what belongs there and offers one action; a prefilled starter set beats a blank form or a tutorial. **Weak**. **For werdy:** first run opens on the starter sets (ADR 0019); every empty report links back to Today.

## Open

* Check the Lally et al. (2010) claim before citing it anywhere user facing.
