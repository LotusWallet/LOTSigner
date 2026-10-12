# P1 Specification — self-audit

| Item | Value |
|---|---|
| Audits | [`P1-SPEC.md`](P1-SPEC.md) §2 and §7 |
| Trigger | A specification that overstates is committing the exact failure it forbids (P1-SPEC §6, §9-4) — so it gets audited like any other claim |
| Method | Read the code; enumerate every `Field<T>` the model exposes; grep for constructors; run the suite |
| Date | 2026-10-12 |
| Result | **2 findings, both open** · 3 non-findings confirmed clean · 1 design question registered · **+1 gap found and closed the same day** (see the addendum) |

> **This audit found a real instance of the failure mode P1-SPEC §9-4 predicts**: a coverage claim
> whose *scope* was wider than its *evidence*. It was found by auditing our own spec one hour after
> writing it — which is the cheapest possible moment to find it.

---

## F1 — `unsupported` is defined but never produced as a field state

**Fact.** `Field::unsupported()` is defined at `crates/lscore/src/state.rs:65`. It has **zero call
sites** anywhere in `lscore`. The `Unsupported` variant is referenced only in `as_str()` and
`explain()`. In practice the fifth state is reached **only** through the `unsupported_keys` list plus
risk rule R001 — never as a per-field badge.

**Consequence.** P1-SPEC §2.1 presents five states as the first property of the deliverable. For
`unsupported`, the state exists in the type system and in the explanation table, but no input can
currently produce it *on a field*.

**Already known?** Partly — [`KNOWN-LIMITATIONS.md`](KNOWN-LIMITATIONS.md) says the same about the UI
("`unsupported` is carried by the named-keys list + R001 rather than a per-field badge"). What was
missing was the same statement in the specification, next to the five-state table.

**Resolution (done).** P1-SPEC §2.1 now states this limit explicitly, in the same paragraph as the
property. No code change: making `unsupported` a field-level badge is a UI/model change, not a
correction of a false statement.

---

## F2 — a test name (and one frozen criterion's wording) claimed more than the test checked

**Fact.** The test `a3_every_field_carries_a_state` — cited in BETA-READINESS §2-A3 as the evidence
that "**Every displayed field** carries one of the five states" — collects states from **three field
kinds only**: `fee_sats`, `sighash_type`, and each input's `amount_sats`.

Displayed items that carry **no** state: `txid`, `version`, `nLockTime` (+ its meaning sentence),
`total_out_sats`, `sighash_policy`; every output's `index` / `value_sats` / `script_type` / `address`
/ `script_hex`; each input's `outpoint` / `sequence` / `script_type` / `partial_sig_count` /
`has_witness_utxo` / `has_non_witness_utxo`. (`OutputFact.value_sats` is a plain `u64` — the type
system offers no state for it.)

**Fair reading.** A3's actual requirement is *"no blanks, no plausible-looking defaults"* — and that
**is** satisfied: nothing is rendered with an invented value, and unknowns are `missing` with a stated
reason. What the wording overstates is the **badge coverage**, not the honesty of the output.

**Resolution (taken).** Make the boundary explicit rather than implied:

1. New test `state_bearing_field_inventory_is_explicit` pins the inventory: its length must equal
   `2 + number of inputs`, in a fixed order, and every member's state must be one of the five.
   If someone adds a state to another field and forgets the inventory, this goes red.
2. `a3_every_field_carries_a_state` carries a scope note naming what it does and does not cover.
3. P1-SPEC §2.1 states the principle: **a state expresses epistemic status relative to a source
   outside the message**, so the three state-bearing fields are exactly those whose value depends on
   data the message only *claims*.

**Honest limitation of the new test** (stated in its doc comment, not hidden): Rust has no reflection,
so it cannot detect a `Field<T>` added to a struct that `collect_field_traces` does not traverse. That
case remains a review obligation. **The test does not pretend to cover it.**

**Not done, deliberately.** BETA-READINESS §7 says the checklist is frozen and that *any amendment
requires re-running the entire list from scratch*. Rewording A3 to "every state-bearing field" would
be an amendment of that weight. Since A3's intent is met and the boundary is now explicit, the
amendment is **left to the project owner** rather than taken unilaterally. §8 below records the
options.

---

## F3 — the boundary principle itself (this is the substantive answer to F2)

A five-state model is only meaningful if it is clear **which** fields it applies to. Stated once, in
one place:

> A state expresses **epistemic status relative to a source outside the message**.
> It applies to a field whose value depends on something the message merely **claims**.
>
> - Input amount — *claimed* through `witness_utxo` / `non_witness_utxo`; only the chain can confirm.
> - Fee — derived from those amounts; inherits their uncertainty.
> - sighash type — a **policy claim** by whoever assembled the PSBT.
>
> Fields **constitutive of the message** — `txid`, `version`, `nLockTime`, output values, output
> scriptPubKeys — are not state-bearing: once the message parses they *are* what was read, and no
> second source exists for them to disagree with. Their check is the parser itself: a malformed
> message is a **hard error**, never a state.

---

## Non-findings (checked, clean)

| Claim | Checked by | Result |
|---|---|---|
| Every risk notice carries a trigger reason and evidence | new `a3_risk_notices_carry_trigger_and_evidence` over the full parseable suite | ✅ holds (`R001`–`R005` all populate all four fields) |
| Layer 4 is never empty | new `a2_layer4_never_empty_and_meaningful` | ✅ always ≥ 5 non-empty entries |
| No fact without a traceable source or stated uncertainty (M1) | new `a5_meta_rule_invariant_across_suite` | ✅ holds — every non-`verified` field carries a note; `parsed_unverified` carries value + note |

---

## Design question registered (owner's call, not taken here)

**Q-D1 — should output values carry a state, or a note?**

- *For adding one:* output value is the single most decision-relevant number on the screen (where the
  money goes). Everything around it carries a badge; it does not.
- *Against:* the output value is not an external claim — it *is* the instruction, read verbatim from
  the message that will be signed. A `[verified]` badge could be *misread* as "verified against
  something"; with no second source, the state would add noise, not information. A plain note
  ("read from the unsigned transaction — this is exactly what you would sign") may be the better form.
- *Why not decided here:* P1-SPEC is a **draft**; adding states to outputs is a model/UI change that
  follows from a spec the owner has not yet adopted. Deciding it now would be the unilateral
  direction change this project's process exists to prevent.

---

## §8 Options left to the project owner (BETA-READINESS A3)

| Option | Effect | Cost |
|---|---|---|
| **A — leave as is** (current state) | A3's intent (no blanks, no invented defaults) is met; the badge boundary is now explicit in P1-SPEC §2.1 and in code | none |
| **B — reword A3** to "every **state-bearing** field" | Removes any wording gap | Per BETA-READINESS §7, **requires re-running the entire checklist from scratch** |
| **C — extend the model** so outputs (and other displayed fields) carry badges | Literal reading of A3 becomes true with no amendment | Model + UI + i18n change; interacts with Q-D1 |

---

## What this audit changed in the repository

| File | Change |
|---|---|
| `crates/lscore/tests/fixtures.rs` | +1 test (`state_bearing_field_inventory_is_explicit`); scope note on `a3_every_field_carries_a_state` |
| `crates/lswasm/src/serialize.rs` | +6 JSON contract tests incl. a zero-dependency JSON validator (addendum F4) |
| `P1-SPEC.md` | §2.1 states the state-bearing boundary and F1's limit; §7 gains A10; Appendix A notes the contract is enforced |
| `g3/evidence/b1-test-log.txt` | regenerated (33 tests) |
| `BUILD.md`, `KNOWN-LIMITATIONS.md`, `BETA-READINESS.md` | test-count and coverage wording kept in step |
| `PROGRESS.md` | §13 records this audit |

*This audit changes no measured fact in `g3/` or `g1/`, and does not amend the frozen checklist. It
records what the specification claimed, what the evidence actually showed, and what was done about
the difference.*

---

## Addendum (same day) — F4: Appendix A's contract was asserted by nothing

Continuing the same question — *which of this spec's claims are backed by something a machine checks?* —
one more gap surfaced, and this one was closed on the spot rather than registered.

**F4 — the front-end JSON contract had no test.** P1-SPEC Appendix A states, as a contract, the exact
JSON shape the web layer consumes. `lswasm` had **zero tests** (0 of the suite's 33 at the time), so
any change to the serializer would have been caught only in a browser — the classic *"it builds,
therefore it works"* gap that this project's own constitution rejects.

**Closed.** Six contract tests now live in `crates/lswasm/src/serialize.rs`:

| Test | What it holds |
|---|---|
| `emitted_json_is_well_formed_for_every_fixture` | every fixture × both languages emits **valid JSON** — checked by a small zero-dependency validator written for the test, not by substring matching |
| `json_contract_matches_p1_spec_appendix_a` | the documented top-level / `facts` / input / output keys exist, and every field object carries the `state` / `value` / `note` / `state_explain` quadruple |
| `risk_objects_carry_all_four_fields` | `code` / `summary` / `trigger_reason` / `evidence` all present |
| `not_provided_and_lang_are_exposed` | layer 4 and the language code are passed through; the strings follow the language |
| `ui_object_exposes_shell_labels` | the shell labels come from the kernel; UI object is itself well-formed JSON |
| `escaping_keeps_json_well_formed` | quotes, backslashes and control characters are escaped (an unescaped quote would break the page) |

**Why the validator matters.** Substring checks would have passed even on malformed output. Writing a
~90-line zero-dependency JSON validator inside the test is what makes the claim *"this is valid JSON"*
actually verified — consistent with the project's refusal to add a dependency for something it can
state precisely itself.

**Suite effect:** 33 → **39** tests. This is recorded as a new P1-SPEC §7 criterion **A10**.

---

## Addendum (same day) — F5·F6·F7: the error channel says things that are not true

This round of self-checking was triggered by a different question than the one above: not *"is this
spec's claim tested?"* but **"is each claim in `BETA-READINESS.md` actually true?"** — verifying the
evidence files and re-running the quoted outputs by hand. That check passed for every claim it covered
(B2's `output[1]` map and byte offset 348 reproduce exactly), but it surfaced a family of defects one
layer down, in the **error channel**. They are recorded here because they are the same species of
problem the meta-rule M1 exists to prevent, and because the specification is what made them visible.

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| **F5** | An error sentence did not follow the requested language: `Display for ReviewError` hard-coded `Lang::En`, so `--lang zh` produced `审阅失败：PSBT parse failed: 报文在…` (English prefix glued onto a Chinese sentence). The same construction also applied the decode prefix **twice**: `Review failed: input decode failed: input decode failed: InvalidChar(33)`. | reproduced on the CLI (four paths × both languages) and independently through the real `.wasm` by instantiating `web/lswasm.wasm` in Node and calling `ls_review` | **Fixed.** `ReviewError` now carries the `Lang` it was built with; `Display` applies the template exactly once. Guarded by `crates/lscore/tests/errors.rs` (6 tests) and 4 FFI-level tests in `crates/lswasm/src/lib.rs`. |
| **F6** | The parser reported "the message ends abruptly inside the global map: a byte was expected at offset N — the PSBT is truncated or corrupt" for a global map that had **terminated normally** and merely omitted the mandatory `PSBT_GLOBAL_UNSIGNED_TX` key. The `Truncated` error was being reused for "a required key is absent". That is a statement about an event that did not happen — an M1 violation in the error channel. | `psbt.rs` `parse_psbt` returned `Truncated { map: "global", offset: i }` where `i` was simply the offset *after* the map's terminating `0x00` | **Fixed.** New `PsbtError::MissingUnsignedTx` with its own string table entry; the `Truncated` variant is now used only where a message really is cut short. A reverse guard test asserts that genuine truncation (S6) is *still* reported as truncation. |
| **F7** | `lib.rs`'s `err_no_global_tx` branch is **unreachable**: `parse_psbt` rejects the condition itself before ever returning `Ok`, so the guard can never fire — and the sentence it would print was never seen by anyone. | reading `parse_psbt` end to end; the branch's condition cannot hold for any returned `Psbt` | **Registered, not deleted.** The guard is retained deliberately: `unsigned_tx_raw` is an `Option`, and a review tool must not `panic` on hostile input. It is now documented as a defensive post-condition re-assertion that the normal path cannot reach, so a future reader is not misled into thinking this string appears in output. The string itself is now reachable — via the parser path that F6 fixed. |

**On scope.** F5 and F6 were fixed rather than merely registered because both are cases where the tool
*says something false*: one sentence in the wrong language, one sentence describing a truncation that
did not occur. For a tool whose entire value proposition is "I read this, I could not read that, I
refuse to conclude the other", an inaccurate error sentence is not a cosmetic defect — it is the same
class of failure as an inaccurate `verified` badge. F7 is a code-hygiene finding with no user-visible
effect, so it is registered with a comment rather than acted on.

**What was *not* changed.** The English wording of every error path that was already correct (bad magic,
genuine truncation, PSBTv2 field, trailing bytes) is byte-identical after the fix — `Truncated`,
`UnsupportedVersion` and `TrailingBytes` still render exactly as before, which is what keeps T0's
canonical baseline untouched (S6 is the only failing fixture and it is a genuine truncation).

### Q-D2 (registered, not adjudicated): how complete is the PSBTv2 detection?

`parse_psbt` recognises a PSBTv2 message by the presence of `GLOBAL_TX_VERSION` (`0x02`) and only that
key. A v2 message that carries `GLOBAL_FALLBACK_LOCKTIME` (`0x03`), `GLOBAL_INPUT_COUNT` (`0x04`) or
`GLOBAL_OUTPUT_COUNT` (`0x05`) but no `0x02` will now be reported as *"the global map does not contain
PSBT_GLOBAL_UNSIGNED_TX"* rather than as *"a PSBTv2 field is present"*. The F6 fix made this message
**accurate** (it does describe what was read), but it is arguably less *diagnostic* than it could be.

Registering rather than deciding, for the same reason as before: the honest choice depends on how much
we want to infer from key-type membership, and "infer the format from a set of related keys" is exactly
the kind of guess this project's constitution tells it not to make silently. Extending the v2 key table
would need its own decision about how many keys must be present before the tool is allowed to *name* a
version.
