# Design tokens

The single source for werdy's look (ADR 0024). Every screen uses these names, never raw values. Light and dark share the same names; a theme is just a different column. Values are taken from the v1 design canvas (ADR 0005). Change a value here first, then in the design and the code.

**See it:** the "Design sheet: tokens and components" frame on the design canvas, https://claude.ai/artifact/WmyN8xF2sjWMGV4AJYTk5y, shows every swatch in light and dark, the font and weights, the type scale, spacing, radii, touch sizes, breakpoints and the core components. If this file and the sheet disagree, this file wins and the sheet is updated.

UI text is not a token: it lives in Fluent files per language and feature, `locales/<lang>/<feature>.ftl` (ADR 0025).

## Colour

| Token | Light | Dark | Used for |
| --- | --- | --- | --- |
| `bg` | `#F4F6F3` | `#0E1411` | Page background (dark is a deep green, not black) |
| `surface` | `#FFFFFF` | `#17201B` | Cards, deed rows, bars |
| `surface-muted` | `#EDF0EC` | `#222C27` | Idle mark buttons, steppers |
| `track` | `#E6EAE6` | `#26312B` | Segmented controls, progress tracks |
| `border` | `#E1E6E2` | `#26312B` | Card borders |
| `border-strong` | `#C9D1CB` | `#2C3832` | Inputs, outline buttons |
| `text` | `#16201B` | `#E7EDE9` | Main text |
| `text-muted` | `#56625B` | `#A2AFA7` | Secondary text (passes 4.5 to 1 on `surface`) |
| `primary` | `#0F5C45` | `#7FD3A8` | Primary buttons, links, active tab |
| `primary-soft` | `#EAF4EE` | `#15281D` | Folded done slots, info notes |
| `on-primary` | `#FFFFFF` | `#08140D` | Text on `primary` |
| `mark-done` | `#1E7A4C` | `#4FB582` | Done mark, good days |
| `on-mark-done` | `#FFFFFF` | `#08140D` | Check icon on Done |
| `mark-partial` | `#E9A92E` | `#F0B64D` | Partial mark, middle days |
| `on-mark-partial` | `#3B2A00` | `#2A1D00` | Half circle icon on Partial |
| `mark-missed` | `#C2413A` | `#E5736A` | Missed mark, low days |
| `on-mark-missed` | `#FFFFFF` | `#1C0705` | Cross icon on Missed |
| `mark-open` | `#8A958E` (outline) | `#7C8983` (outline) | Open deeds and today in progress, always dashed or outlined, never filled |
| `danger` | `#A3332C` | `#E5736A` | Destructive text (stop sharing, delete) |
| `overlay` | `#16201B` | `#000000` | Day note dialog shade |

Every mark has its own shape too (check, half filled circle, cross, dashed outline), so colour is never the only signal.

## Score bands

| Token | Value | Meaning |
| --- | --- | --- |
| `band-good` | 80% | At or above: `mark-done` colour |
| `band-mid` | 50% | At or above, below good: `mark-partial` colour; below: `mark-missed` colour |

## Type

| Token | Value |
| --- | --- |
| `font-family` | IBM Plex Sans Arabic (covers Arabic and Latin); Noto Sans Arabic to compare, `___` |
| `line-height-body` | 1.5 |
| `size-caption` | 13 px |
| `size-body` | 16 px (never below for body text) |
| `size-label` | 15 px |
| `size-title` | 17 px |
| `size-screen` | 24 px (phone), 26 px (tablet) |
| `size-score` | 48 px |
| `weight-regular`, `weight-medium`, `weight-semibold`, `weight-bold` | 400, 500, 600, 700 |

## Size and space

| Token | Value |
| --- | --- |
| `touch-min` | 48 px for every interactive control |
| `rail-width` | 96 px (navigation rail on tablet and wide web) |
| `gap-xs`, `gap-s`, `gap-m`, `gap-l` | 4, 8, 12, 16 px |
| `page-gutter` | 16 px (phone), 28 px (tablet) |
| `radius-control` | 14 px |
| `radius-card` | 16 to 18 px |
| `radius-pill` | half the height |
| `stroke-thin`, `stroke-strong` | 1 px, 2 px | Card edges and outlined day cells |

## Layout breakpoints (ADR 0022)

| Token | Value | Layout |
| --- | --- | --- |
| `bp-medium` | 600 dp | Below: phone, one column, bottom tabs |
| `bp-expanded` | 840 dp | From here: two panes and a navigation rail |
| `content-max` | `72rem` | Content width cap on a wide web window. Provisional. ___ Owner to confirm. |
| `dialog-max-width` | 36 rem | Maximum width of the day note dialog |
| `overlay-opacity` | 45% (light), 65% (dark) | Tint of the shade behind the day note dialog |
