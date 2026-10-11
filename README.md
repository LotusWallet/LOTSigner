# Lotus Signer

**English** · [中文版 README_CN.md](README_CN.md)

> Before you sign what someone else handed you, run it through Lotus Signer.

**Lotus Signer is a local-first review and signing-collaboration tool for data you are asked to sign.**

When someone hands you a piece of data to sign, Lotus Signer reconstructs the transaction facts locally,
marks what is missing and what is risky, lets you confirm, and then you sign with **your own** wallet or
hardware. It never custodies a key, and there is no Lotus-operated business server.

> **The core value is providing independent verification evidence — a second opinion — not explaining the
> data for you.** Semantic translation is a required capability, but it is not the value itself: the
> translation layer is being commoditised into infrastructure and is easy to copy. **Explicitly stating
> "this information is missing", "here is the boundary of what was verified", and "these two tools
> disagree" requires admitting you do not know, which commercial products are structurally unwilling to
> do. Honesty has a moat; translation does not.**

> **Commitments come in four layers**: L-A absolute ｜ L-B scoped (for clearly supported, complete PSBTs:
> review → sign → export, entirely offline) ｜ L-C enhanced (risk annotation etc., **prominently labelled
> "requires network"**, data source chosen by the user) ｜ L-D explicitly not committed.
> **We never use blanket phrasing like "always offline" or "never connects".**

## Status

| Item | Value |
|---|---|
| Current stage | **P1-0 technical validation gates** (2 weeks / 1 person) — order **G3 → G1 → G2**, G2 deferrable |
| Verdict so far | Direction holds. **P1 scope is not pre-locked**; the spec is written only after G1–G3 produce results |
| Execution | **G3 and G1 can start immediately**; G2 awaits hardware |
| Primary scenario | BTC PSBT entrusted review + signature return + export (stateless, no wallet management) |
| Target user | Technical users who are comfortable with self-custody and multisig, can judge whether a transaction matches their intent, **but will not sign on the strength of a raw PSBT or a single wallet's UI alone** (deliberately narrow) |
| Effort | 1–2 core developers; the stop-loss line is **moved forward to P1-0** |
| Nature of the project | **Technical-credibility asset + strategic option**; not measured by revenue KPI |

**The repository is currently private. See [Openness policy](#openness-policy) — we do not open a
half-finished, untestable build.**

## Product Constitution

1. **Handle only "what is being signed" and "the result of signing". Never handle accounts or broadcast.**
2. Never create, custody, or touch a user's private key.
3. No Lotus-operated business server.
4. **Trustworthy code distribution is a P0 requirement (5 non-negotiable goals) and a separate release
   gate. It is not met by a deadline and is never cut for schedule.**
   - **G-I** the user can independently verify the version, integrity and provenance of the artifact they
     run (two paths: self-build comparison / consuming the official artifact; the latter must be ready
     within P0)
   - **G-II** no outbound requests inside committed scenarios ｜ **G-III** key parsing behaviour is
     reproducible ｜ **G-IV** release-authority boundaries and disclosure ｜ **G-V** the user can
     self-host
   Concrete mechanisms (IPFS/CID, SRI, build tooling, release platform) may be swapped after security
   review, but the evidence must be re-established and never changed unilaterally.
5. Unsupported formats and fields **must raise explicit errors**. No silent degradation, no filling gaps
   with guesses.
6. Every critical field carries one of five states: `verified` / `parsed_unverified` / `missing` /
   `unsupported` / `check_failed`. **`verified` means only "passed a clearly defined parse and
   validation rule" — it never means "this transaction is verified safe".**
7. Output has four layers: transaction facts / contextual explanation / risk notes (with the trigger
   shown) / conclusions we do not provide. **We never emit a final "safe/dangerous" verdict.**
   (The five states describe field evidence status; the four layers describe information types. They are
   not interchangeable.)
8. **Single authoritative semantic implementation.** Every judgement affecting transaction facts or
   review state has exactly one implementation (Rust core → native CLI + wasm32 web). The web layer may
   contain presentation, input adaptation and file-handling code, but **must not re-implement
   authoritative parsing, transaction-fact decisions, or the five-state logic**; adaptation layers that
   could change a conclusion are covered by tests. Externally, T4 independent cross-checks must use an
   **independently sourced** implementation, with CI inspecting the dependency tree to keep the
   cross-check library out of production builds.

## Explicitly Out of Scope

In-house MPC · own hardware device · browser extension · mobile (before P3) · code-level integration
with the LotusWallet main product (before P3) · token narrative

## Repository Layout

```
g3/                 P1-0 gate G3: pre-registration, fixtures, tools, evidence
packages/           source (core / parsers / risk / transport / web / cli)
ANNOUNCEMENTS.md    outbound announcement queue (published manually)
PROGRESS.md         progress board (single entry point)
BETA-READINESS.md   the frozen checklist that decides when we may open the repo
```

> **The deliberation documents are not in this repository.** Drafts and verdicts from the four review
> rounds were archived separately by the project owner and removed from this repo **and from the entire
> git history** on 2026-10-11 (history rewritten). The conclusions live in **Product Constitution** and
> **Explicitly Out of Scope** above — **this README is the authoritative source for external reference.**

## Engineering Documents

| Document | Description |
|---|---|
| [`PROGRESS.md`](PROGRESS.md) | Progress board (single entry point) |
| [`BETA-READINESS.md`](BETA-READINESS.md) | **Frozen checklist: when we are allowed to open the repo** |
| [`ANNOUNCEMENTS.md`](ANNOUNCEMENTS.md) | Outbound announcement queue |
| [`g3/G3-预注册-Caravan重合度实测.md`](g3/G3-预注册-Caravan重合度实测.md) | G3 pre-registration (frozen; must not change after testing) |
| [`g3/G3-实测记录-阶段1.md`](g3/G3-实测记录-阶段1.md) | G3 phase-1 test log (including environment pitfalls) |
| [`g3/fixtures/README.md`](g3/fixtures/README.md) | PSBT fixtures and dual-verification results |

## Process Rules

1. **Gate pre-registration**: before each gate, five elements are frozen in writing (supported scope /
   critical fields / must-reject errors / difference rules / reproduction method) and may not change
   after testing.
2. **Meta-principle**: **engineering feasibility must never substitute for market validation, and user
   demand must never substitute for security validation. Both classes of evidence must stand on their
   own.**
3. **This file is authoritative**: the deliberation documents have been archived out of the repo; the
   Product Constitution and Explicitly Out of Scope sections are the final conclusions.
4. *(Archived)* Version-fingerprint and citation-verification rules were used to prevent version
   incidents during the deliberation rounds; no longer mandatory now that deliberation has ended.

## About LotusWallet Team

**LotusWallet Team** initiates Lotus Signer, developed as an independent product and treated as a
technical-credibility asset and acquisition funnel for the team. It feeds back into the main product at
stage P3.

## Outbound Channels

> Decision by the project owner, 2026-10-11: **GitHub and X are the primary channels for interacting
> with users. GitHub carries the content, X publishes the news, and a copy is mirrored to a Telegram
> group. The company itself is serverless — anything that needs to be distributed or pushed is carried
> as an open-source GitHub repository.**

This lifts Constitution item 3 ("no Lotus-operated business server") from the **product layer** to the
**organisational layer**: the tool we build and the way we operate are the same thing.

| Layer | Channel | Carries | Publishing |
|---|---|---|---|
| **Content** | **GitHub** | README, roadmap, docs, release notes, security advisories. **Must be self-contained** — readable without X or Telegram | git commit |
| **Broadcast** | X | announcements | **manual** (low frequency) |
| **Broadcast** | Telegram group | mirror of X posts | **manual** (low frequency) |

X and Telegram are broadcast only, never a source of truth. Making the GitHub content self-contained is
the only mitigation for channel-concentration risk.

**No API integration, no Telegram bot** (project owner, 2026-10-11: publishing is not high-frequency).
Outbound content is written into [`ANNOUNCEMENTS.md`](ANNOUNCEMENTS.md) first and published manually by
the project owner. Bots and automation are deferred.

### ⚠️ Release discipline (non-negotiable, from Constitution item 4)

**Code being public ≠ artifact being releasable.** Content can be published at any time; a runnable
artifact must wait until it can be signature-verified.

| Layer | Contents | Gate |
|---|---|---|
| Repository readable | source, README, roadmap, docs | no P0 gate |
| **Release attachments / any runnable artifact** | binaries, wasm, dist, installers | **T0 green + signing-key three requirements**, otherwise upload is prohibited |
| Early access | **source + self-build instructions only** (G-I path A) | must be labelled **not verified by official signature** |

Signing-key three requirements: ① key held offline / in an isolated environment ② public-key
fingerprint written into a git signed tag **from day one** ③ documented key-compromise procedure (if you
cannot write it, you are not ready to ship).

**The product must not embed "announcement fetch" or "version check" at runtime.** Security advisories
and version information are distributed via GitHub Security Advisory + Release + CHANGELOG.

## Remote Repository

GitHub: `https://github.com/LotusWallet/LOTSigner`

⚠️ **Spelling**: `LOTSinger` (appearing in some earlier notes) is a typo. The correct name is
**`LOTSigner`** (Lotus + Signer, no "g").

| Item | Value |
|---|---|
| Owner | `LotusWallet` (personal account, not an org) |
| Push identity | `BingLeague` (as collaborator) |
| Visibility | **private** |

## Openness Policy

**We do not open the repository while the product is an untestable half-finished state. Opening
something the community cannot meaningfully test is irresponsible.**

Visibility is therefore driven by **a build that can actually be tested plus sign-off by the project
owner** — never by a date on a calendar. The criteria are fixed in advance in
**[`BETA-READINESS.md`](BETA-READINESS.md)** and must not be relaxed after the fact.

### Making the repository public and publishing artifacts are two different acts

**A public repository ≠ a published official artifact.** Conflating them would delay openness by
roughly two weeks for no reason.

| Act | Requires | Note |
|---|---|---|
| **Making the repo public** (source readable) | buildable code | readable source needs no signature |
| **Publishing an official artifact** (release attachment / web bundle) | **T0 green + signing-key three requirements** | Constitution item 4; never cut for schedule |

**→ Recommended: Track 1 first, Track 2 after.**

- **Track 1 (fast)**: make the repo public; the beta is offered as **source + one-command build**
  (G-I path A). Compliant **without T0 and without signing**. Cost: only people who can build it
  themselves can use it — **which is exactly the P1 target audience** (technical users comfortable with
  self-custody and multisig).
- **Track 2 (required, later)**: once T0 (cross-build artifact consistency test) and the signing-key
  three requirements are ready (~1–2 person-days), publish **official artifacts** for users who will not
  build it themselves.

**Track 2 must be complete before any non-beta distribution.**

**Either way, "testable" is not a judgement call** — it is the frozen checklist in
[`BETA-READINESS.md`](BETA-READINESS.md).

### Delivery forms during beta (to limit spread)

| Recipient | Form | Labelling requirement |
|---|---|---|
| Project owner | preferably **builds it themselves from the script**; a prebuilt artifact may be supplied when needed | must be labelled **internal test build · not verified by official signature · do not redistribute** |
| Beta users (Track 1) | repo source + build instructions | instructions must state **no officially signed artifact exists yet** |
| Beta users (Track 2) | official release artifact + signature + verification instructions | full G-I path B |

## Deliberation Documents Removed (done 2026-10-11)

The project owner decided on 2026-10-11 to **delete all deliberation documents**, avoiding copyright
issues and keeping this repository purely engineering.

Two steps, both required:

1. `git rm -r doc` — removes the working tree and future commits;
2. **rewrite the entire history** — otherwise the drafts remain intact inside `.git` in earlier commits,
   and once public anyone could recover all of them with `git show <old-commit>:doc/xxx.md`.
   **Removing without rewriting is equivalent to not removing anything.**

> `g3/顾问A-第五轮分类确认.md` was removed as well (same class of copyright risk).
> Drafts are backed up outside the repo, and are **not in git**.

**Consequence**: the chain of reasoning is no longer visible externally. The conclusions are fixed in
**Product Constitution** and **Explicitly Out of Scope** above; the reasoning stays in the team's
internal archive. **This README is the single authoritative source for external reference.**
