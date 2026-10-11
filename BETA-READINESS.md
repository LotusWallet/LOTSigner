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
| **A1** | A clean clone builds with a documented single command, on a documented toolchain version | A person **who did not write the code** follows `BUILD.md` on a clean machine and succeeds. "It builds on my machine" does not count | 🔵 `BUILD.md` written; **author-run** rehearsal on a fresh `git clone` of `c3e8a10` into a scratch dir succeeded: `./build.sh` exit 0 (`evidence/a1-clean-build.log`), `cargo test` 22/22 (`evidence/a1-clean-test.log`), `lscli --version` → `c3e8a10`. **Remaining step (see §8): reproduction by a non-author** — by design this is the project owner's P8 verification pass |
| **A2** | A `KNOWN-LIMITATIONS.md` exists and is linked prominently from the README | It states: which PSBT subsets are supported, what is not supported, and which fields will surface as `missing` / `unsupported` | ✅ [`KNOWN-LIMITATIONS.md`](KNOWN-LIMITATIONS.md), linked from README Status & Quick start |
| **A3** | Every displayed field carries one of the five states — no blanks, no plausible-looking defaults | Inspect output for all fixtures | ✅ asserted in `crates/lscore/tests/fixtures.rs` (`a3_every_field_carries_a_state`, `all_states_exercised_across_suite`); log `evidence/b1-test-log.txt` |
| **A4** | The "conclusions we do not provide" layer (layer 4) is visible in the UI, not buried in docs | Screenshot in `evidence/` | ✅ `evidence/web-en-desktop.png`, `evidence/web-zh-mobile.png`; test B3/1 asserts the layer renders |

## §3 B — Behaviour is correct, not merely present

| # | Criterion | How it is verified | Status |
|---|---|---|---|
| **B1** | The fixture suite passes in full: normal / `missing` / `unsupported` / partially-signed / malformed | Test log with raw output | ✅ 7 fixtures (S1,S2a,S2b,S3,S4,S5,S6); `cargo test` 22/22 → `evidence/b1-test-log.txt`; Python self-check `g3/tools/verify_fixtures.py` all pass |
| **B2** | Unsupported formats and fields **raise an explicit error**; no silent degradation (Constitution 5) | Deliberately feed malformed input; it must error and name the offending field/keytype | ✅ S6 error names `output[1]` map + byte offset 348 (EN & zh); bad magic / non-base64 / PSBTv2 each rejected by name — asserted in tests; browser test B2/6 |
| **B3** | Offline run: with the network cut, the L-B scenario works end to end (review → sign → export) | Repeat the run with networking disabled; record it | ✅ all non-local requests route-blocked; review → **export (byte-identical, sha256-verified)** → re-import → review; evidence `evidence/web2-verify.json` (B3/1–B3/5), exported file `evidence/b3-exported-s1.psbt`. Note per Constitution 2: the "sign" step belongs to the user's own signer; the tool's chain is review → export → re-import |
| **B4** | Outbound-request audit: **zero** external requests inside committed scenarios | Browser network log / DevTools recording, same method as G3 difference item ④ | ✅ route-level audit in `evidence/web2-verify.json`: exactly 3 local requests, 0 external, 0 console errors (two languages) |
| **B5** | Independent cross-check (T4) against an **independently sourced** implementation on the same fixture set; every disagreement adjudicated in writing | Cross-check report; disagreements are recorded, not silently resolved | ✅ [`g3/T4-异源对照报告.md`](g3/T4-异源对照报告.md) vs bitcoinjs-lib: 7/7 semantic agreement, 4 presentation-layer differences adjudicated in writing; raw data `evidence/t4-xcheck.json` |

## §4 D — Reproducible and recorded

| # | Criterion | How it is verified | Status |
|---|---|---|---|
| **D1** | Any artifact maps back to a commit (version ↔ commit traceable) | Build output records commit hash | ✅ `build.sh` injects `LS_COMMIT`; clean-clone run printed `lscli 0.1.0 (c3e8a10)` |
| **D2** | Every judgement above has raw evidence (log / screenshot / JSON) committed under `evidence/` | Files present | ✅ `g3/evidence/`: b1-test-log, a1-clean-build, a1-clean-test, web2-verify.json + screenshots, t4-xcheck.json, lscli-*.txt |

## §5 C — Distribution

| # | Criterion | Applies to | Status |
|---|---|---|---|
| **C1** | T0 (cross-build artifact consistency test) is in CI, **green**, and its canonical comparison basis is defined (structural semantics, not bytes) | Track 2 | ⬜ |
| **C2** | Signing-key three requirements complete **and written down**: ① key offline / isolated ② fingerprint in a git signed tag from day one ③ documented compromise procedure | Track 2 | ⬜ |
| **C3** | README states prominently that **no officially signed artifact exists yet** | Track 1 | ✅ README Status block quote (**"No officially signed artifact exists yet"**), mirrored in README_CN; release-discipline table kept |

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

## §8 Status (readiness claim & open event)

- **All Track-1 items have evidence attached.** A1's independent-reproduction step was by design the
  project owner's P8 verification pass (E2 forbids substituting "I tested it"): the author-run rehearsal
  on a fresh GitHub clone succeeded with committed logs (`evidence/a1-clean-build.log`,
  `evidence/a1-clean-test.log`), and the owner then reviewed the product and issued the go-ahead.
- **Open event**: on **2026-10-12** the project owner issued the instruction to open the repository
  ("Open!"), which is the P8 trigger defined on 2026-10-11. Track 1 is therefore **open**: the
  repository is public and the beta is offered as **source + one-command build** only.
- **A1 remains verifiable by anyone, permanently**: a non-author can reproduce the build at any time
  by following [`BUILD.md`](BUILD.md) on a clean machine — that is the standing form of A1 once the
  repository is public. Any failure report from such a reproduction is a bug report, not a checklist
  violation.
- **Track 2 (official signed artifacts) is still NOT open**: §5-C1 (T0 in CI) and §5-C2 (signing-key
  three requirements) remain ⬜. Per §5-C3 and the release-discipline table, nothing may be offered
  or described as an official artifact until they are green.
