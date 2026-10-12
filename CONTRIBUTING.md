# Contributing

Thanks for looking. Before you spend time on a patch, here is what this project optimises for — it
is unusual in a few places on purpose.

## The one rule that shapes everything

**There is exactly one implementation of the authoritative semantics, and it must never say something
we cannot stand behind.**

- All parsing, transaction-fact determination, five-state logic and risk notices live in
  `crates/lscore`. The CLI (`crates/lscli`) and the wasm build (`crates/lswasm`) are thin shells.
- The web page may contain **presentation, input handling and file plumbing only**. If you find
  yourself computing a fact, a state or a risk in JavaScript, that logic belongs in the core.
- Third-party PSBT libraries are welcome as **cross-check references** (see T4 in
  `g3/T4-异源对照报告.md`) and must never enter the parsing path — a second parser would quietly
  nullify the independent cross-check.

Two things worth internalising from the Constitution (README §Product Constitution):

1. **`verified` does not mean "safe".** It means "passed the defined parsing and verification rules".
   A change that makes output *feel* more reassuring without new evidence is a regression.
2. **Unsupported input must raise an explicit error.** No silent degradation, no plausible-looking
   defaults, no guessing.

## Before you open a PR

```bash
./build.sh && cargo test --release --locked
python3 g3/tools/verify_fixtures.py        # independent of cargo
```

- Small, focused commits with a message explaining *why*.
- New behaviour comes with a fixture (or a test) that fails before the change.
- Any change to i18n strings: add the key to **every** shipped language in
  `crates/lscore/src/i18n.rs`. Language additions are welcome, but a language is only enabled after
  native-speaker proofreading — a mistranslated risk notice misleads users.
- If you touch parsing, run the T0 dump locally (`bash g3/tools/t0_dump.sh g3/fixtures /tmp/t0 target/release/lscli`)
  and be ready for CI to compare it across three platforms.

## Adding a language

Adding a language = adding a `Lang` variant and a match arm table in `crates/lscore/src/i18n.rs`; the
CLI (`--lang`) and the web page pick it up automatically. Ship it enabled only if you are a native
speaker or can name the proofreader in the PR description.

## What we will say no to

- A second parser or a "quick fix" in the web layer.
- Presenting a conclusion the Constitution forbids (a "safe/unsafe" verdict, a blanket "always
  offline" claim, marketing-voice reassurance).
- Anything that phones home, embeds an update check, or fetches announcements at runtime. Our
  outbound channel is GitHub (Release / Advisory / CHANGELOG) — the product itself stays quiet.

## Security issues

Do not open a public issue — see [`SECURITY.md`](SECURITY.md).

## License

Contributions are accepted under **MIT OR Apache-2.0** (see README §License).
