# Security Policy

Lotus Signer is a tool people point at data they are **about to sign**. A wrong answer from this
tool is worse than no answer. We would rather hear about a flaw early.

## Reporting a vulnerability

- **Preferred**: open a **private security advisory** on GitHub
  (Security tab → *Report a vulnerability*). That keeps the report out of public view until a fix is
  published.
- **Do not** open a public issue for anything that could cause someone to sign the wrong thing
  (a parser that silently accepts malformed input, a state that renders `verified` when it should be
  `missing` / `check_failed`, an amount or address rendered incorrectly, a code path that reaches out
  to the network, …).
- Please include: fixture or input bytes (base64 is fine), the exact command or button path, the
  output you got, the output you expected, and the commit hash (`lscli --version` prints it).

## What counts as a security bug here

| Class | Examples |
|---|---|
| **Silent degradation** | Anything accepted without an explicit error where the Constitution demands one (Constitution 5) |
| **State inflation** | `verified` shown for something that only passed a weaker check; a blank or plausible-looking default instead of `missing` |
| **Wrong facts** | Incorrect txid, amount, address, fee, sighash or byte offset |
| **Hidden network access** | Any outbound request inside a committed scenario (Constitution / G-II) |
| **Divergence between implementations** | CLI and web disagreeing, or two builds behaving differently (this should be impossible: one core, tested by T0/T4) |

By contrast, the following are **not** security bugs, though we want to hear about them anyway:
unsupported formats being rejected (that is the design), cosmetic issues, and disagreements with
another wallet's presentation layer.

## What we will do

- Acknowledge the report, reproduce it with a fixture, and fix it with a regression test that fails
  before the fix and passes after.
- Disclose in plain language — what was wrong, for how long, what is affected — in
  [`ANNOUNCEMENTS.md`](ANNOUNCEMENTS.md) and in the release notes. No downplaying.
- Credit the reporter unless they prefer otherwise.

## Signing keys

Artifact-signing policy (key custody, day-one fingerprint commitment, compromise procedure) is in
[`SIGNING.md`](SIGNING.md). **No official artifact is signed yet** — do not trust binaries claiming
to be from this project.

## Scope

This policy covers this repository (the Rust core `lscore`, CLI `lscli`, wasm build, and the web
page). It does not cover third-party wallets, coordinators or hardware devices, even when used
alongside Lotus Signer.
