# KNOWN-LIMITATIONS

> **Read this before using the review output.** A beta that hides its own boundary
> manufactures false confidence (BETA-READINESS E5). This file states exactly where the
> tool's understanding stops. Last updated: see git history.

## What this tool is

An **evidence-based review surface** for PSBTs that someone else asked you to sign.
It parses the message offline, labels every displayed field with one of five evidence
states, explains context, lists triggered risk rules with their reasons — and
**never** tells you a transaction is safe or dangerous, never touches private keys,
and never goes online.

## Supported input

- **PSBT v0 only** (BIP-174 with `PSBT_GLOBAL_UNSIGNED_TX`).
  PSBT v2 fields (`PSBT_GLOBAL_TX_VERSION` etc.) are **rejected with an explicit error** —
  the tool does not guess at v2 semantics. (Fixtures: S6 covers rejection.)
- Unsigned transactions in the classic (non-witness) serialization with empty `scriptSig`s,
  up to 10,000 inputs/outputs.

## What is intentionally *not* implemented (beta scope)

| Area | Status | What you will see |
|---|---|---|
| **Signature verification** | Not implemented | Existing `partial_sig`s are counted and surfaced (R004) but **never cryptographically verified** |
| **Segwit address rendering beyond v0** | Not implemented | Taproot (P2TR) outputs are *classified* and rendered via bech32m, but taproot *spending rules* are not modeled |
| **PSBT v2** | Rejected explicitly | Error names the offending key type |
| **Online verification** | Never | Input amounts from `witness_utxo` are `parsed_unverified`; the fee note states this explicitly |
| **xpub / descriptor parsing** | Global xpubs are recorded as uninterpretable keys (`unsupported_keys`), not parsed |
| **GUI signing, key management, wallet** | Out of scope by constitution — the tool hands the PSBT back unchanged (`--export` / export button) |

## How unsupported things surface

Per Constitution §5–§7 there is **no silent degradation and no guessing**:

- Unknown/proprietary key types → listed **by name** in `unsupported_keys`, risk rule
  **R001** fires with the key names as evidence.
- Missing previous-transaction data → input amount is `missing`, fee is `missing`
  (**never 0**), risk rule **R002** fires.
- Fields that exist but fail a defined check (e.g. outputs exceed inputs; truncated
  sighash field) → `check_failed`, risk rule R003 fires, the trigger reason is displayed.
- Malformed/truncated messages → **hard error** naming the map (`input[0]`, `output[1]`…)
  and the byte offset. The tool never renders a partial guess.

## Field-level state meanings

| State | Meaning |
|---|---|
| `verified` | Parsed by explicitly defined rules and passed structural checks (e.g. amount read from a complete attached previous transaction whose txid matches the outpoint) |
| `parsed_unverified` | Read out of the message; its correctness against the chain was **not** verified (e.g. amounts from `witness_utxo`) |
| `missing` | Not provided in the message; no guess is filled in |
| `unsupported` | Present but not interpretable — surfaced as named keys + R001 (field-level badge reserved; see `unsupported_keys` array) |
| `check_failed` | Present, known type, but failed a defined check |

## Where the five states are exercised

All five are exercised by the committed fixture suite (S1–S9) and asserted in
`crates/lscore/tests/fixtures.rs`. `unsupported` is carried by the named-keys list +
R001 rather than a per-field badge in the current beta UI.

## The one rule we hold ourselves to (M1)

> **Every factual statement in the output must be traceable to something we actually read in
> the message — or it must be labelled with the source of its uncertainty.**

This is stated as a rule rather than a promise because it came from a **measured failure in
another tool**: a review surface that reported `This address has a zero balance.` when the
truth was *"the lookup failed"*. Writing "I could not find it" as "it does not exist" is
precisely the failure this rule forbids.

What you can check, rather than take on faith:

- A field that is unknown is `missing` **with a stated reason** — never a value, never `0`.
  In particular, if any input amount is unknown, the **fee is `missing`**, not `0`.
- A field read out of the message but not checked against the chain is `parsed_unverified`,
  with the note saying why it is unverified.
- A field that failed a check is `check_failed` and **carries no value** — nothing is carried
  over from a value we just said we cannot trust.
- Layer 4 ("what this tool does NOT tell you") is present for **every** input — it is never
  empty and never collapses into a normal-length document.

These four are enforced as tests over the whole fixture suite (see
[`P1-SPEC.md`](P1-SPEC.md) §7 A2/A3/A5), not left to reviewer discipline.

## Known cosmetic/behavioural notes

- The fee is computed **only** from amounts attached to the PSBT itself; if any input
  amount is unknown the whole fee is `missing` — this is deliberate.
- `nLockTime` interpretation (block height vs timestamp) uses the standard 500,000,000
  threshold; the interpretation is shown next to the value.
- Addresses are rendered **mainnet only**; testnet PSBTs will render mainnet-style
  addresses for standard scripts (a known cosmetic limitation).
- Language coverage: **English (default) and 中文**. Further languages (ja/ko/fr/th) are
  architecturally ready but will not ship without native-speaker review — a wrong risk
  translation is misinformation.

## How to verify these claims yourself

```bash
cargo test --offline                       # 32 tests incl. BIP-173 official vectors
python3 g3/tools/verify_fixtures.py        # fixture self-check (independent Python parser)
node g3/tools/xcheck_t4.js                 # cross-check vs bitcoinjs-lib (see g3/T4-异源对照报告.md)
```

## Fields we recognise but do not use (transparency)

Standard BIP-174 fields such as `PSBT_GLOBAL_XPUB` and `PSBT_IN_BIP32_DERIVATION` are **recognised as
standard** and are therefore **not** reported as unknown. They do not enter the review: knowing which
master fingerprint a key belongs to changes nothing about what the transaction *says*, and we do not
use it to decorate output. Only genuinely uninterpretable values — proprietary (BIP-174 `0x0f`) keys
and key types outside the standard tables — raise the R001 "fields this tool cannot interpret" notice.
(Until 2026-10-12 the standard fields above were wrongly counted as unknown, which produced a false
R001 on almost every PSBT exported by a coordinator; see `g3/fixtures/README.md`.)
