# Dependency security log

## 2026-10-22: crossbeam-epoch 0.9.18 -> 0.9.21 (RUSTSEC-2026-0204)

- Ticket: `cargo audit` reported RUSTSEC-2026-0204 on `crossbeam-epoch 0.9.18` (pulled in by `rayon` through `crossbeam-deque`). Fixed in `>=0.9.20`.
- Change: lockfile-only, `cargo update -p crossbeam-epoch --precise 0.9.21` (newest 0.9.x). `Cargo.toml` is untouched.
- Result: `cargo audit` reports zero vulnerabilities for this advisory. The remaining `rand 0.8.5` RUSTSEC-2026-0097 is an `unsound` warning (custom logger calling `rand::rng()`), reported only and not a vulnerability here.
- Validation: `cargo test` on the primary checkout, same pass count before and after.
- Redeploy: none needed; the library ships as a wasm/native artifact, rebuild on next release.
