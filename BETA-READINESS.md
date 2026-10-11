# Beta Readiness — when we are allowed to open this repository

> **We do not open a half-finished build that nobody can meaningfully test. Opening that would be
> irresponsible to the community.**
> — project owner, 2026-10-11

The word "testable" is a judgement call until it is written down. This file is that writing-down: it is
**pre-registered**, in the same sense as the G3 gate pre-registration. **These criteria are fixed before
the work starts and must not be relaxed afterwards.**

- **Canonical:** English. Mirrored in [`README_CN.md`](README_CN.md) §公开策略.
- **Status legend:** ⬜ not started ｜ 🔵 in progress ｜ ✅ evidence attached ｜ ❌ failed

---

## §1 Two tracks, two thresholds

| Track | What opens | Extra requirements |
|---|---|---|
| **Track 1** | Repository public; beta offered as **source + one-command build** | §2 + §3 + §4 + **§5-C3** |
| **Track 2** | **Official signed artifacts** published (release attachment / web bundle) | Track 1 + **§5-C1** + **§5-C2** |

**Track 2 is mandatory before any non-beta distribution.** Track 1 may go first; it does not need T0 or
signing, because it ships no official artifact.

---

## §2 A — Anyone can verify it (no insider knowledge)

| # | Criterion | How it is verified | Status |
|---|---|---|---|
| **A1** | A clean clone builds with a documented single command, on a documented toolchain version | A person **who did not write the code** follows `BUILD.md` on a clean machine and succeeds. "It builds on my machine" does not count | ⬜ |
| **A2** | A `KNOWN-LIMITATIONS.md` exists and is linked prominently from the README | It states: which PSBT subsets are supported, what is not supported, and which fields will surface as `missing` / `unsupported` | ⬜ |
| **A3** | Every displayed field carries one of the five states — no blanks, no plausible-looking defaults | Inspect output for all fixtures | ⬜ |
| **A4** | The "conclusions we do not provide" layer (layer 4) is visible in the UI, not buried in docs | Screenshot in `evidence/` | ⬜ |

## §3 B — Behaviour is correct, not merely present

| # | Criterion | How it is verified | Status |
|---|---|---|---|
| **B1** | The fixture suite passes in full: normal / `missing` / `unsupported` / partially-signed / malformed | Test log with raw output | ⬜ |
| **B2** | Unsupported formats and fields **raise an explicit error**; no silent degradation (Constitution 5) | Deliberately feed malformed input; it must error and name the offending field/keytype | ⬜ |
| **B3** | Offline run: with the network cut, the L-B scenario works end to end (review → sign → export) | Repeat the run with networking disabled; record it | ⬜ |
| **B4** | Outbound-request audit: **zero** external requests inside committed scenarios | Browser network log / DevTools recording, same method as G3 difference item ④ | ⬜ |
| **B5** | Independent cross-check (T4) against an **independently sourced** implementation on the same fixture set; every disagreement adjudicated in writing | Cross-check report; disagreements are recorded, not silently resolved | ⬜ |

## §4 D — Reproducible and recorded

| # | Criterion | How it is verified | Status |
|---|---|---|---|
| **D1** | Any artifact maps back to a commit (version ↔ commit traceable) | Build output records commit hash | ⬜ |
| **D2** | Every judgement above has raw evidence (log / screenshot / JSON) committed under `evidence/` | Files present | ⬜ |

## §5 C — Distribution

| # | Criterion | Applies to | Status |
|---|---|---|---|
| **C1** | T0 (cross-build artifact consistency test) is in CI, **green**, and its canonical comparison basis is defined (structural semantics, not bytes) | Track 2 | ⬜ |
| **C2** | Signing-key three requirements complete **and written down**: ① key offline / isolated ② fingerprint in a git signed tag from day one ③ documented compromise procedure | Track 2 | ⬜ |
| **C3** | README states prominently that **no officially signed artifact exists yet** | Track 1 | ⬜ |

---

## §6 Must-reject errors

Any of these invalidates the readiness claim, even if every checkbox above is ticked.

| # | Error |
|---|---|
| **E1** | Treating "it builds" as "it is testable" — a binary that runs but whose behaviour nobody can describe is a half-finished build |
| **E2** | Substituting "I tested it" for "someone else can reproduce it" — A1 requires an independent person |
| **E3** | Relaxing this checklist after the fact (the "revise-and-pass" move under schedule pressure) — this is exactly the failure mode pre-registration exists to prevent |
| **E4** | Treating "it produced output" as "the output is correct" — without B5 there is no correctness evidence |
| **E5** | Omitting or burying known limitations — a beta that hides its own boundary manufactures false confidence |
| **E6** | Calling something an official artifact before C1 and C2 are green |
| **E7** | Treating "the page opens with the network unplugged" as "the L-B scenario is offline end to end" — B3 requires the full chain |
| **E8** | Filling unknown fields with guesses (Constitution 5) |

---

## §7 Adjudication

- **Track 1 requires**: all of §2, §3, §4 and §5-C3.
- **Track 2 requires**: Track 1 plus §5-C1 and §5-C2.
- **One item failing means we do not open / do not publish.** There is no partial credit.
- **This checklist is frozen.** Any amendment must be recorded in writing with its reason, and amending
  it requires re-running the entire list from scratch.
