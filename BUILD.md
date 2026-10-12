# BUILD — one command from a clean clone

> BETA-READINESS **A1**: a person who did not write the code follows this file on a clean
> machine and succeeds. If any step fails for you, that is a bug in this file — please
> [open an issue](https://github.com/LotusWallet/LOTSigner/issues) with the exact error output.

## Toolchain (tested versions)

| Component | Version tested | Notes |
|---|---|---|
| Rust (stable, rustup) | **1.99.0** (any recent stable ≥ 1.85 should work) | MSVC toolchain on Windows; default on Linux/macOS |
| `wasm32-unknown-unknown` target | installed automatically by `build.sh` | — |
| Python 3 | any 3.9+ | **optional** — only to re-generate fixtures |
| Node.js | **not required** | the web page uses the raw `.wasm` directly, no bundler; the test suite needs no Node either |

**No third-party crates.** The workspace has zero dependencies; the build never touches
crates.io. `Cargo.lock` exists but pins nothing external.

## Build

```bash
git clone https://github.com/LotusWallet/LOTSigner
cd LOTSigner
./build.sh
```

`build.sh` does exactly three things:

1. `cargo build --release --offline -p lscli` → `target/release/lscli(.exe)`
2. `cargo build --release --offline --target wasm32-unknown-unknown -p lswasm` → `web/lswasm.wasm`
3. copies the wasm and sample fixtures into `web/demo/`

The commit hash is injected at build time (`LS_COMMIT`); `lscli --version` reports it
(BETA-READINESS D1: every artifact maps back to a commit).

On Windows without bash, the equivalent manual steps:

```bat
set CARGO_NET_OFFLINE=true
cargo build --release --offline -p lscli
rustup target add wasm32-unknown-unknown
cargo build --release --offline --target wasm32-unknown-unknown -p lswasm
copy target\wasm32-unknown-unknown\release\lswasm.wasm web\
```

## Run

**Web** (recommended — this is the product surface):

```bash
cd web
python3 -m http.server 4180     # Windows: double-click web\serve.cmd
```

Open <http://127.0.0.1:4180>, click **Load sample**, or drop any base64 `.psbt` file.
The page makes **zero external requests** — verified automatically, see `g3/evidence/web2-verify.json`.

**CLI:**

```bash
target/release/lscli g3/fixtures/S1.psbt            # review (English)
target/release/lscli --lang zh g3/fixtures/S1.psbt  # review (Chinese)
target/release/lscli --export out.psbt g3/fixtures/S1.psbt   # export unmodified PSBT
```

## Test

```bash
cargo test --offline
```

49 tests, in four groups:

- **Kernel unit tests** (incl. BIP-173 official vectors) — `crates/lscore/src/`
- **Fixture-suite integration tests** — `crates/lscore/tests/fixtures.rs`: the P1 meta-rule
  invariant (a non-`verified` field must state its uncertainty source), the layer/risk
  contracts and an explicit inventory of which fields are state-bearing
- **Error-channel invariants** — `crates/lscore/tests/errors.rs`: the error sentence must be
  in the requested language (no English template fragment surviving in a Chinese error), the
  prefix must be applied exactly once (no `input decode failed: input decode failed: …`), a
  message that merely *lacks a required key* must not be described as *truncated*, and a
  genuinely truncated message must still be described as truncated
- **Front-end contract** — `crates/lswasm/src/serialize.rs` checks that the emitted JSON is
  well-formed for every fixture in both languages; `crates/lswasm/src/lib.rs` calls
  `ls_review` through the same FFI sequence the browser uses (`ls_alloc` → `ls_review` →
  read the 4-byte length prefix), so the **web** error path is covered too — no Node needed

Full log: `g3/evidence/b1-test-log.txt`.

Optional, and only after `build.sh` has produced a wasm binary — check the **artifact** the browser
actually loads, not just the source:

```bash
node g3/tools/check_wasm_error_lang.js     # defaults to web/lswasm.wasm
```

## Fixture suite

`g3/fixtures/` contains ten PSBTs covering the five field states, a malformed message,
and two real-world shapes: S1 normal · S2a `missing` · S2b `unsupported` · S3 partially
signed · S4 P2SH · S5 `check_failed` · S6 truncated (must be rejected) · S7 previous
transaction with a non-empty scriptSig (the on-chain normal case) · S8 previous-transaction
txid deliberately mismatched · S9 a **third-party tool's real export** (Caravan).
Regenerate the synthetic ones with `python3 g3/tools/make_psbt_fixtures.py`, self-check with
`g3/tools/verify_fixtures.py`.

> S7–S9 exist because hand-built fixtures only test the cases we thought of: both were
> exposed by feeding the tool a real export from another tool. See
> [`g3/fixtures/README.md`](g3/fixtures/README.md).

All fixtures are **synthetic**: no real funds, no real xpubs, no real keys. Public keys
are multiples of the secp256k1 generator (mathematically valid, owned by nobody).
