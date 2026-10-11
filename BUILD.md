# BUILD — one command from a clean clone

> BETA-READINESS **A1**: a person who did not write the code follows this file on a clean
> machine and succeeds. If any step fails for you, that is a bug in this file — please
> [open an issue](../../issues) with the exact error output.

## Toolchain (tested versions)

| Component | Version tested | Notes |
|---|---|---|
| Rust (stable, rustup) | **1.99.0** (any recent stable ≥ 1.85 should work) | MSVC toolchain on Windows; default on Linux/macOS |
| `wasm32-unknown-unknown` target | installed automatically by `build.sh` | — |
| Python 3 | any 3.9+ | **optional** — only to re-generate fixtures |
| Node.js | **not required** | the web page uses the raw `.wasm` directly, no bundler |

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

22 tests: kernel unit tests (incl. BIP-173 official vectors) + the fixture-suite
integration tests (`crates/lscore/tests/fixtures.rs`). Full log: `g3/evidence/b1-test-log.txt`.

## Fixture suite

`g3/fixtures/` contains seven synthetic PSBTs covering the five field states plus a
malformed message: S1 normal · S2a `missing` · S2b `unsupported` · S3 partially signed ·
S4 P2SH · S5 `check_failed` · S6 truncated (must be rejected). Regenerate with
`python3 g3/tools/make_psbt_fixtures.py`, self-check with `g3/tools/verify_fixtures.py`.

All fixtures are **synthetic**: no real funds, no real xpubs, no real keys. Public keys
are multiples of the secp256k1 generator (mathematically valid, owned by nobody).
