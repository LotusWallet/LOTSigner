# P1 Specification — the boundary model as a verifiable public good

> **Status: DRAFT for project-owner review.** Written 2026-10-12, immediately after gates G1 and G3
> concluded. It was deliberately not written earlier: the standing rule is *"P1 scope is not
> pre-locked; the spec is written only after G1–G3 produce results"* (README §Status).
>
> **Provenance.** The direction is fixed by `g3/G3-投入产出重新评估-备忘.md` §5.4 — *"from 'filling
> Caravan's gaps' to 'making the boundary model a verifiable public good'"*. That memo is itself the
> action the G3 pre-registration mandates for its own verdict ("conditional pass → re-assess ROI").
> **The order of these documents is visible in git history; nothing here is a retro-fitted
> justification.** The external facts referenced below were verified directly against the GitHub /
> npm APIs on 2026-10-12 (see PROGRESS §11.5), not recalled from memory.
>
> **This is not a gate pre-registration.** It has no frozen checklist to be gamed, and it is not a
> claim of readiness. It becomes binding only when P1 development starts; until then it is a proposal.

---

## §1 What P1 is

P1 delivers a **local-first, offline, evidence-based review surface for PSBTs that someone else asked
you to sign** — as a *verifiable reference implementation of a boundary model*, not as a
feature-competitive wallet.

That distinction is the whole point of the re-scope, and it is stated plainly here so it cannot drift:

| | A differentiated product would say | P1 says |
|---|---|---|
| Value claim | "other tools cannot do X" | **"here is what we read, what we could not read, and what we refuse to conclude"** |
| Success measure | adoption, revenue | **is the boundary model verifiable by a stranger, and is it honest** |
| Where effort goes | closing gaps competitors have | **making every claim traceable to an artifact anyone can re-run** |

**Why the claim is not about competitors.** G3 tested three assumptions that Lotus Signer would be
differentiated by (no-config import / partially-signed export / xpub privacy). **All three were
disproven by our own measurements** — see `g3/G3-判定表-实跑结果.md`. What survived were two
*quality* gaps, and the ROI memo showed both are cheap for anyone else to close. So P1 has no
competitor-comparative claim to make, and **must not make one** (§6).

**The user, unchanged** (README): technical users comfortable with self-custody and multisig, who can
judge whether a transaction matches their intent, **but will not sign on the strength of a raw PSBT or
a single wallet's UI alone**.

---

## §2 The deliverable: the boundary model

The deliverable is not a screen. It is a **model with four testable properties**. Everything else —
CLI, web page, i18n, fixtures — exists to make these four properties *checkable by someone who did not
write the code*.

### 2.1 Property 1 — every displayed field carries an evidence state

Five states, and only five. They live in the data model, not in the UI
(`crates/lscore/src/state.rs`, `Field<T>`).

| State | Means exactly | Does **not** mean |
|---|---|---|
| `verified` | parsed by explicitly defined rules and passed the structural checks defined for it | **never** "this transaction is safe" |
| `parsed_unverified` | read out of the message; its correctness against the chain was not checked here | not "confirmed" |
| `missing` | not provided in the message | not "zero", not "empty", not "nothing there" |
| `unsupported` | present, but this tool cannot interpret its meaning | not "invalid", not "ignorable" |
| `check_failed` | present, known type, failed a defined check | not "malicious" |

**Binding rule.** No displayed field may be blank and no field may carry a plausible-looking default.
A field that is unknown is `missing` with a stated reason; it is never rendered as a value.

### 2.2 Property 2 — output has four layers, and layer 4 is never empty

`crates/lscore/src/state.rs` — `Review`.

| Layer | Field | Content |
|---|---|---|
| 1 Transaction facts | `facts` | txid, version, nLockTime (+ its meaning), per-input facts, per-output facts, total out, fee, sighash |
| 2 Contextual explanation | `context` | what the numbers mean *in this situation* — never advice |
| 3 Risk notices | `risks` | rule code, summary, **trigger reason**, **evidence** — all four fields mandatory |
| 4 **Conclusions we do not provide** | `not_provided` | **always ≥ 5 entries; never empty in any input** |

Layers are not interchangeable with states: **states describe a field's evidence status; layers
describe the kind of information.** Conflating them is a defect (README Constitution 7).

### 2.3 Property 3 — every risk notice shows why it fired

| Code | Fires when | Evidence field must contain |
|---|---|---|
| **R001** | fields present that this tool cannot interpret | the **names** of the offending keys (e.g. `PSBT_IN_PROPRIETARY`) |
| **R002** | fee is unknown (an input carries neither `witness_utxo` nor `non_witness_utxo`) | the per-input amount states |
| **R003** | outputs exceed inputs in total value | the computed total output |
| **R004** | an input already carries a signature | the explicit statement that signatures are **not** cryptographically verified |
| **R005** | an output scriptPubKey matched no known standard form | the script hex |

**A risk notice with an empty `trigger_reason` or empty `evidence` is a defect, not a style issue.**
The whole reason a notice exists is to let a reader disagree with it.

> **Correction carried forward from a real defect.** Until 2026-10-12, standard BIP-174 fields
> (`PSBT_GLOBAL_XPUB`, `PSBT_IN_BIP32_DERIVATION`) were counted as unknown, so **almost every real
> PSBT raised a spurious R001** — which trains users to ignore warnings. Fixed, and locked by
> fixtures S9 + `proprietary_still_reported_after_s9_fix`. The lesson is structural: **a notice that
> fires when it should not is as harmful as one that fails to fire.**

### 2.4 Property 4 — the meta-rule (M1)

> **M1.** Every factual statement in the output must be traceable to *"what I read"* — a specific
> field or byte range — or must be explicitly labelled with the source of its uncertainty.
>
> **Violations, stated as the shapes they take:**
> - *"I could not find it"* written as *"it does not exist"*;
> - *"not verified"* written as *"confirmed"*;
> - a value rendered where the message supplied none;
> - a failure to fetch rendered as a fact about the world.

**M1 is not a slogan.** It came from a concrete, measured failure in another tool: G3 difference item ②
observed a "zero balance" message produced when the real situation was "the query failed". That is the
exact failure M1 forbids, and it is why M1 is the **first hard constraint** of P1 (§3, C0) and is
carried by an **executable invariant** (§7, A5) rather than by intent.

**Enforceable form** (the part a machine can check):

1. In the data model: a `Field<T>` whose state is not `verified` **must** carry a non-empty `note`
   naming the source of the uncertainty; `parsed_unverified` must carry a value *and* a note.
2. In the strings: every user-facing sentence comes from `crates/lscore/src/i18n.rs`, so M1 review is
   a review of one file — and every sentence must state its evidence or its uncertainty.

---

## §3 Hard constraints (inherited from the Product Constitution)

P1 adds nothing to these and may not weaken them.

| # | Constraint | Where it lives |
|---|---|---|
| **C0** | **M1** (§2.4) — no fact without a traceable source or a stated uncertainty | this spec, enforced by A5 |
| **C1** | Handle only "what is being signed" and "the result of signing"; never accounts, never broadcast | Constitution 1 |
| **C2** | Never create, custody, or touch a private key; never sign | Constitution 2 |
| **C3** | No Lotus-operated business server | Constitution 3 |
| **C4** | **Zero outbound requests inside committed scenarios** | Constitution 4 (G-II) |
| **C5** | Unsupported formats and fields raise explicit errors — **no silent degradation, no guessed fills** | Constitution 5 |
| **C6** | Fifth state is never a verdict: we never emit "safe" or "dangerous" | Constitution 6–7 |
| **C7** | **Single authoritative semantic implementation** — one Rust core; the web layer may adapt input and present, but must not re-decide facts or states | Constitution 8 |
| **C8** | Artifact trust is a separate gate, satisfied by evidence and never cut for schedule | Constitution 4 |

---

## §4 Scope of P1

| # | In scope | State today |
|---|---|---|
| S1 | PSBT v0 review: transaction facts, five states on every displayed field, four-layer output | working (`lscore`) |
| S2 | Two front ends over the same core: native CLI (`lscli`) and an offline single-page web app (`web/`) | working |
| S3 | Hand the input back **unmodified** (`--export` / export button); signing stays in the user's own signer | working, byte-identical (sha256-verified) |
| S4 | i18n: **EN (default) + zh-CN**, all user-facing text from the kernel's string table | working |
| S5 | A committed fixture suite that exercises every state and every risk rule | 10 fixtures (S1–S9 + malformed) |
| S6 | T0 cross-build consistency in CI (three platforms) | green (Track 2 / C1) |
| S7 | T4 independent cross-check with **written adjudication of every disagreement** | 10/10, adjudications D1–D2 |
| S8 | Self-build path (G-I path A) documented so a stranger can reproduce the build | `BUILD.md` |

**The P1 deliverable is the model plus the evidence that the model holds** — not the UI polish.

---

## §5 Explicitly out of P1

| Not in P1 | Why |
|---|---|
| **PSBT v2** | Rejected with an explicit error naming the key type. Guessing at v2 semantics would violate C5 |
| **Cryptographic signature verification** | Out by constitution (C2-adjacent): the tool never derives keys. Existing signatures are counted and surfaced (R004) and **stated as unverified** |
| **Taproot spending rules** | P2TR outputs are classified and rendered via bech32m; *spending* semantics are not modelled |
| **Online verification of any kind** | C4. Amounts from `witness_utxo` are `parsed_unverified`; the fee note says so explicitly |
| **GUI signing, key management, wallet** | C1/C2. The tool hands the PSBT back unchanged |
| **BDK / any enhancement layer in the core** | Blocked on the constitutional questions X1/X2 (§8), not on feasibility. G1 established that BDK-wasm *can* work offline; adoption is a different question |
| **Hardware-wallet integration (G2)** | Awaits hardware |
| **Official signed artifacts** | Track 2 (§7, A6) — a separate gate, not a P1 feature |

---

## §6 What P1 must not claim (wording discipline — hard)

The ROI memo §5.1 makes this a hard constraint. It exists because we disproved our own differentiation.

**Forbidden, in any outward-facing text (README, announcements, issues, comments):**

1. Any suggestion that another tool *cannot* do something — no "competitors can't", no "no other tool",
   no "the only tool that" (E5-style extrapolation).
2. Any blanket offline phrasing: no "always offline", no "never connects". The commitment is scoped
   (L-A/L-B), and layer L-C is labelled "requires network" when it exists.
3. Any "official" wording while no signed artifact exists (README release discipline).
4. Any unverified external fact stated as verified.

**Permitted form** — describe what was observed, in the committed scenario, with the evidence path:

> *"In that no-configuration import path, the tool did not present the transaction outputs needed to
> make a signing decision, and it stated a failed lookup as a zero balance."*
> — a statement of a measured phenomenon, attributable to `g3/G3-判定表-实跑结果.md`.

---

## §7 Acceptance criteria

P1 is **done** when all of the following hold, each with committed raw evidence. A criterion without an
artifact is not met.

| # | Criterion | How it is checked | Source of the requirement |
|---|---|---|---|
| **A1** | Every displayed field carries one of the five states; no blanks, no defaults | `crates/lscore/tests/fixtures.rs` asserts it across the suite | BETA-READINESS A3 |
| **A2** | Layer 4 (`not_provided`) is non-empty for **every** input, including malformed ones that parse | asserted in the same suite | this spec §2.2 |
| **A3** | Every risk notice carries a non-empty `trigger_reason` **and** `evidence` | asserted in the same suite | this spec §2.3 |
| **A4** | The five states are all exercised by the committed suite; `unsupported` is carried by the named-keys list + R001 | asserted across the suite | BETA-READINESS A3 / B1 |
| **A5** | **M1 executable invariant**: every `Field` with a non-`verified` state carries a non-empty note; `parsed_unverified` carries value + note; a missing input amount forces the fee to `missing` with **no value** (never 0) | asserted across the suite | this spec §2.4 (ROI §5.3) |
| **A6** | T0 (cross-build artifact consistency) green in CI on ubuntu/windows/macos, comparison on structural semantics | CI workflow + green run link | Constitution 4 / C1 |
| **A7** | T4 independent cross-check on the full fixture set, **every** disagreement adjudicated in writing | cross-check report | BETA-READINESS B5 |
| **A8** | The suite contains **at least one artifact exported by a third-party tool** and one real-world transaction shape | fixtures S7–S9 | G3 lesson (§9.3) |
| **A9** | A stranger can build and run it from a clean clone by following one document | `BUILD.md`, standing A1 | BETA-READINESS A1 |

> **A8 is a P1-level requirement on purpose.** It generalises the single most valuable thing G3
> produced: *fixtures we construct can only test the situations we thought of.* Two real defects —
> a strict-parse rule wrongly applied to a previous transaction, and standard BIP-174 fields reported
> as unknown — were invisible to every hand-built fixture and were found only by feeding the tool a
> **third-party tool's real export**. A suite that contains no foreign artifact does not meet this spec.

---

## §8 Open questions — registered, **not decided here**

These are constitutional questions. This spec **records** them and does not rule on them; a
specification is not the place to settle a constitution, and the author of a gate report must not
adjudicate the gate's own outcome.

| # | Question | Facts on the table (2026-10-12) |
|---|---|---|
| **X1** | **Zero-dependency core vs. BDK dependency tree.** `lscore` is currently zero-runtime-dependency | Introducing BDK takes the wasm bundle from **≈89 KB to ≈2.3 MB (≈23×)** and adds a C toolchain requirement to self-build (G1 Q1b) |
| **X2** | **Single authoritative semantics vs. a second parser.** `CONTRIBUTING.md` forbids a second parser | BDK **can** independently parse descriptors and derive addresses (G1 Q3, cross-checked to address level) — i.e. real semantic overlap with `lscore` |

**Consequence for P1:** neither question blocks P1 as scoped in §4, because **P1 does not include the
enhancement layer** (§5). That is precisely why G1 could be de-scoped to a half-day probe with no veto
power before it ran.

---

## §9 What would falsify this spec

A specification that cannot be wrong is not a specification. This one is falsified, wholly or partly,
by any of the following — and the correction is a **new file**, never an edit to this one.

1. **A field is displayed without one of the five states, or with a guessed fill.** → §2.1 is broken;
   the fix is in the code and a new fixture, and this must be reported as a defect, not smoothed over.
2. **A risk notice fires without a trigger or evidence, or fires when it should not.** → §2.3 broken
   (the R001 false-positive of 2026-10-12 is an instance of the second half).
3. **An output sentence states something as known when it was not read or not verified.** → §2.4 (M1)
   broken. This is the failure mode M1 exists to catch, and finding one is a *success of the rule*,
   not an embarrassment.
4. **The fixture suite contains no foreign artifact and a third-party export exposes a defect.** →
   §7-A8 was not actually met; the suite's coverage claim was overstated.
5. **P1 is pursued as a competitor-comparative product** (a differentiation claim reappears in outward
   text). → §1 and §6 were abandoned; the correct response is to stop and re-assess, not to reword.

---

## Appendix A — the machine-readable contract

The web layer consumes exactly this shape and performs **no** parsing or state decisions (C7). Every
field object carries its state, its note, and a human-readable explanation of the state *rendered by
the kernel* — so the front end cannot re-word a state (Constitution 8, extended).

```jsonc
{
  "ok": true,
  "lang": "en",                      // "en" | "zh-CN"
  "ui": { /* shell labels only — no semantic sentences */ },
  "facts": {
    "txid": "<64 hex>", "version": 2, "locktime": 0,
    "locktime_meaning": "<kernel-rendered>",
    "inputs": [{
      "index": 0, "outpoint": "<txid>:<vout>", "sequence": 4294967295,
      "script_type": "P2SH" | "P2WSH" | "P2SH-P2WSH" | "script type not labelled" | null,
      "amount": { "state": "verified", "value": 1500000, "note": "…", "state_explain": "…" },
      "partial_sig_count": 0,
      "has_witness_utxo": true, "has_non_witness_utxo": false
    }],
    "outputs": [{ "index": 0, "value_sats": 1200000, "script_type": "P2WSH",
                  "address": "bc1…" | null, "script_hex": "<hex>" }],
    "total_out_sats": 1490000,
    "fee_sats":      { "state": "parsed_unverified", "value": 10000, "note": "…", "state_explain": "…" },
    "sighash_type":  { "state": "verified", "value": 1, "note": null, "state_explain": "…" },
    "sighash_policy": "<kernel-rendered explanation>"
  },
  "context": ["…"],                                       // layer 2
  "risks": [{ "code": "R002", "summary": "…",
              "trigger_reason": "…", "evidence": "…" }],  // layer 3 — all four mandatory
  "not_provided": ["…", "…", "…", "…", "…"],              // layer 4 — never empty
  "unsupported_keys": ["input[0]/PSBT_IN_PROPRIETARY:…"]  // named, never summarised
}
```

On failure the core returns an explicit error naming the offending **map** (`input[0]`, `output[1]`, …)
and the **byte offset** — never a partial guess (C5).

---

## Appendix B — how to verify every claim in this file

```bash
cargo test --offline                      # fixture suite: states, layers, risk fields, M1 invariant
python3 g3/tools/verify_fixtures.py       # independent Python fixture self-check
node g3/tools/xcheck_t4.js                # T4 cross-check vs bitcoinjs-lib (see g3/T4-异源对照报告.md)
bash g3/tools/t0_dump.sh g3/fixtures out target/release/lscli   # T0 canonical output, one platform
```

| Claim in this file | Verify with |
|---|---|
| §2.1 five states, §2.2 four layers | `crates/lscore/src/state.rs` |
| §2.3 risk codes and their evidence | `crates/lscore/src/lib.rs`, string table in `crates/lscore/src/i18n.rs` |
| §2.4 M1 invariant | test **A5** in `crates/lscore/tests/fixtures.rs` |
| §7-A6 T0 green | `.github/workflows/ci.yml` + the green run linked in BETA-READINESS §5-C1 |
| §7-A7 T4 adjudications | `g3/T4-异源对照报告.md` |
| §7-A8 foreign artifact in the suite | `g3/fixtures/README.md` (S9 provenance) |
| §6 wording discipline | `ANNOUNCEMENTS.md` writing rules, items 1–7 |
| §8 X1/X2 facts | `g1/G1-判定记录-BDK-WASM离线能力实测.md` §4–§5 |

---

*This is a draft specification, not a readiness claim and not a gate pre-registration. It will become
binding when P1 development starts. It changes none of the measured facts in `g3/` or `g1/`; those
remain the authoritative record of what was tested.*
