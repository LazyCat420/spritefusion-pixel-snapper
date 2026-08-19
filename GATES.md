# Gates (`scripts/ops/`)

Landed 2026-08-19. This repo previously had no test or check script; these are
its first.

```bash
scripts/ops/scoped-gate.sh    # gates covering this tree's changes
scripts/ops/full-suite.sh     # everything, in sequence
scripts/ops/scoped-gate.sh --self-test    # prove the exit-status plumbing
```

## Why a single-crate repo gets a cross-target leg

`Cargo.toml` splits its dependencies on `target_arch`: `rayon` natively,
`wasm-bindgen` + `getrandom` on wasm. Two dependency sets behind a `cfg` is
exactly the shape that builds green on one target and breaks on the other, and
the `cdylib` is the shipped artifact for the web tool — so the native tests
passing tells you very little about what users run.

So the gate always runs `cargo check --lib --target wasm32-unknown-unknown`
whenever the closure is non-empty. `--lib` is load-bearing: the `[[bin]]`
target pulls in the native-only CLI path and would fail the wasm check for a
reason that has nothing to do with the shipped library.

## Enforcement: everything votes here

Measured on `main` before choosing, the way every repo in this family sets it:

| leg | on `main` 2026-08-19 | enforcing? |
|---|---|---|
| `cargo fmt --check` | GREEN | yes |
| `cargo clippy --all-targets` | GREEN | yes |
| `cargo test --no-fail-fast` | GREEN | yes |
| `cargo check --lib --target wasm32` | GREEN | yes |

This repo is clean, so nothing is advisory. (Sibling repos start fmt/clippy
advisory precisely because they are red on `main`, and a gate that is red the
day it ships trains people to ignore it.)

## The exit code is the product

Every leg's command runs **bare** — never `cmd | grep`, never `cmd | tail`. A
trailing pipe makes `$?` the filter's, which is how a suite elsewhere reported
four real reds as exit 0. `--self-test` proves the plumbing in one command: it
re-execs with a green leg, a leg exiting 42, and a red **behind a pipe**, then
asserts the child failed and named both reds.

Exit codes: `0` green · `1` a leg failed · `3` a structural change (add,
delete, rename) selected no gate — the dangerous shape, `--allow-empty`
downgrades it · `10` setup failure · `75` the box was full and **nothing ran**,
which is not a red suite.

## Shared machinery

`gate-lib.sh`, `crate-map.py`, `scoped-gate.sh` and `full-suite.sh` are
identical copies across four sibling Rust repos; only `gate-config.sh` is
local. `GATE_LIB_VERSION` (currently **3**) is the drift tripwire — fix the
library, carry it to all four, bump the version.

The library also takes CPU thread locks from a machine-global pool, so a gate
run here cannot take the whole shared box, and it leases a shared
`CARGO_TARGET_DIR` when free (falling back to a private one, and always saying
which it got).

## Open items

- Nothing outstanding. All four legs are green and enforcing on `main`.
