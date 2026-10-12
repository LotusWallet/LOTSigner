# Signing & Release Key Policy

> This document is part of BETA-READINESS §5-C2 (signing-key three requirements). The three
> requirements are: ① the key is generated and kept offline / in an isolated environment;
> ② the public-key fingerprint is committed to a git **signed tag from day one**;
> ③ a documented compromise procedure exists (this document is it).
>
> **Status: policy adopted. The key ceremony has NOT been performed yet. Until it is, no artifact
> may be called official — see the release-discipline table in the README.**

## Why this exists

Code distribution trust (Constitution goal G-I) has two paths. Path A — *build it yourself from
source* — is available today and needs no signature. Path B — *consume an official artifact* —
becomes available only when artifacts are signed and the signing story is verifiable. This policy
governs Path B.

It also states, up front, what a signature does **not** prove:

| A signature proves | A signature does NOT prove |
|---|---|
| This artifact was produced by the holder of this key | The key holder is trustworthy |
| The artifact was not modified after signing | The source code itself is not malicious |
| (with reproducible builds) the artifact matches reviewed source | Deterministic build success = this artifact is the official one |

A signature from us is a *transport integrity* guarantee, nothing more. The independent second
opinion remains the product's own review output — never a signature.

## 1. Key generation and custody (requirement ①)

- **Algorithm & tooling**: GnuPG, Ed25519 (`gpg --quick-generate-key "LotusWallet Team (release signing) <release@lotuswallet.example>" ed25519 sign`), expiry set to a fixed date and renewed on schedule.
- **Generation environment**: an air-gapped machine (or a live USB session with networking off). The private key **never touches a networked machine**. The ceremony is performed by the project owner and, if possible, witnessed — the raw output of the ceremony (fingerprint, revocation certificate creation) is recorded in a signed tag message or release note.
- **Day-to-day signing**: signing of release artifacts is done by exporting the result, or via a hardware token (YubiKey / smartcard) holding the subkey. The primary key stays offline permanently.
- **Backup**: one encrypted backup, physically separate from the working machine, plus the printed revocation certificate. Losing the key without a published revocation certificate is itself a compromise event (see §3).

## 2. Day-one fingerprint commitment (requirement ②)

- Immediately after key generation, the owner creates a **git signed tag** (e.g. `signing-key-v1`) whose tag message contains:
  - the full fingerprint;
  - the key creation date;
  - a pointer to this document.
- The tag is pushed before any signed artifact exists. This is the "from day one" requirement: the repository itself, under git's signature machinery, is the canonical record of which key is ours — not any webpage, X post, or download portal.
- Users verify with: `git verify-tag signing-key-v1`, then compare the printed fingerprint against the one in the tag message (two independent channels: the tag's own signature and the fingerprint embedded in it; a mismatch is an alarm).
- Rotating to a new key requires a new signed tag referencing the old fingerprint and this document, plus an announcement. Old tags are never deleted.

## 3. Compromise procedure (requirement ③)

A compromise event is any of: private key (or signing token) loss, suspected key exposure, a
signature we cannot account for, or loss of the key without a revocation certificate.

1. **Revoke immediately.** Publish the revocation certificate to keyservers and commit it to the
   repository (`keys/revocation-<fingerprint>.asc`) in the same push.
2. **Disclose plainly, no spin.** A note in `ANNOUNCEMENTS.md`, a GitHub Security Advisory, and a
   Release note, stating: what happened, when it was noticed, what is affected, what is not.
   Consistent with the product's honesty policy: no downplaying, no "out of an abundance of caution".
3. **Re-key.** Generate a new key per §1, tag it per §2 with an explicit reference to the revoked
   fingerprint. Everything signed with the compromised key after the last verified-good artifact is
   considered untrusted and is re-signed.
4. **Retrace.** List every artifact ever signed by the compromised key, state for each whether it
   was built from a reviewable commit, and whether a re-signed replacement exists.
5. **Post-mortem.** How did it happen, why did detection take as long as it did, what changes.
   Published alongside the disclosure.

## 4. What becomes possible once this is done (Track 2)

- Official signed artifacts may be published (release attachments, wasm bundle) with per-artifact
  detached signatures and SHA256SUMS signed in one blob.
- BETA-READINESS §5-C1 (T0 in CI, green) must **also** be green first — the two gates are
  independent and both mandatory. A signed artifact whose build is not consistency-tested is just
  a signed lottery ticket.

## 5. Current status

| Requirement | Status |
|---|---|
| ① Offline custody procedure written | ✅ this document |
| ① Key ceremony performed (offline generation, hardware token) | ⬜ owner action, offline |
| ② Day-one signed tag published | ⬜ blocked by ① |
| ③ Compromise procedure written | ✅ this document, §3 |
| §5-C1 T0 in CI green | see BETA-READINESS.md |

Until all rows are ✅: **there are no official artifacts. Anything claiming to be one is not from
this project.**
