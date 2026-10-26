# Domain docs

How the engineering skills read werdy's domain documentation. werdy is a single context repo, with two deliberate differences from the skills' default layout.

## Before exploring, read these

* **`CONTEXT.md`** at the repo root: the glossary. werdy uses this name in place of `GLOSSARY.md`.
* **`docs/decisions.md`**: all ADRs in one file, newest first, numbered `0001` upward. werdy uses this in place of a `docs/adr/` folder.
* **`docs/context/`**: background copied from the owner's research (the Werd app's behaviour, the owner).

## Writing them

* A new glossary term goes into `CONTEXT.md` in the format from the `domain-modeling` skill: bold term, one or two sentences, and an `_Avoid_` line for rejected synonyms. No implementation details.
* A new ADR goes at the top of `docs/decisions.md` as `## NNNN (YYYY-MM-DD): Title`, with the next free number. Say the decision, why, and what would make us revisit it.
* All prose follows the writing rules in `AGENTS.md` (no dashes joining words).

## Use the glossary's vocabulary

When output names a domain concept (an issue title, a test name, a proposal), use the term as `CONTEXT.md` defines it. If the term is missing, either it is invented language (reconsider) or a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If output contradicts an ADR, say so explicitly rather than silently overriding it:

> _Contradicts ADR 0004 (three marks per deed), but worth reopening because..._
