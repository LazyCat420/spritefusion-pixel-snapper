#!/usr/bin/env bash
# gate-config.sh — spritefusion-pixel-snapper's gate facts. Single crate, so
# the closure logic degenerates to "the crate, or nothing". Sourced, never run.

GATE_REPO_NAME="spritefusion-pixel-snapper"
GATE_CLIPPY_ARGS=""

# Set by MEASUREMENT on main, not preference — see the landing commit.
GATE_FMT_ENFORCE=1
GATE_CLIPPY_ENFORCE=1

GATE_TRIGGERS=()

# The cdylib is the SHIPPED artifact (wasm-bindgen), and the crate splits its
# dependencies on target_arch: rayon natively, wasm-bindgen/getrandom on wasm.
# Two dependency sets behind a cfg is exactly the shape that builds green on
# one target and breaks on the other.
GATE_TARGETS=("wasm32-unknown-unknown")
GATE_WASM_PKGS="spritefusion-pixel-snapper"

gate_target_entry() { echo "spritefusion-pixel-snapper"; }
gate__target_list() { echo "$GATE_WASM_PKGS"; }
gate_target_pkgs() { shift; echo "spritefusion-pixel-snapper"; }
gate_target_full_pkgs() { echo "spritefusion-pixel-snapper"; }
gate_target_flag() { echo wasm; }

# The wasm leg must be --lib: the [[bin]] pulls in the native-only CLI path.
gate_target_extra_args() { [ "$1" = "wasm32-unknown-unknown" ] && echo "--lib"; }

gate_extra_legs()       { :; }
gate_extra_legs_names() { echo "(none)"; }
gate_full_extra_legs()  { :; }
