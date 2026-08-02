"""
PARITY WITH THE PIPELINE THIS WAS EXTRACTED FROM.

A port that merely runs is not a port. These pin the Python against numbers the
TypeScript produced on three real generated sheets, so a rewrite that quietly
changes behaviour fails here rather than in someone's game.

Skipped when the reference sheets are not present, so the package stays
standalone. Point `SPRITEFORGE_REF_SHEETS` at a directory of `<name>-S.png` plus
`<name>-S.json` (a manifest with `rows[].cells`) to run them.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

from spriteforge.grid import detect_pixel_grid
from spriteforge.matte import matte, rgb_hex
from spriteforge.gutter import gutter_report
from spriteforge.slice import slice_sheet_detail

REF = Path(
    os.environ.get(
        "SPRITEFORGE_REF_SHEETS",
        "/home/lazycat/github/projects/sun/braindeadbot-client/public/sprites",
    )
)

# (best factor, its confidence %, cell purity %) as the TypeScript reported them.
#
# `flat_share` is deliberately NOT pinned here: the port measures it over OPAQUE
# pairs only, where the TS counted every pair. See `_flat_share` — the TS number
# depended on an in-memory buffer keeping RGB under alpha=0, which a PNG round
# trip destroys, and the divergence fixes a latent gate bug rather than causing
# one. What must not drift is the factor, the purity and the VERDICT.
EXPECTED = {
    "jester": (5, 0.4, 5.9),
    "beaver": (10, 0.3, 2.2),
    "frog": (4, 2.1, 7.9),
}


def _load(name: str) -> tuple[np.ndarray, list[tuple[int, int, int, int]]]:
    manifest = json.loads((REF / f"{name}-S.json").read_text())
    img = np.asarray(Image.open(REF / f"{name}-S.png").convert("RGBA"))
    boxes = [tuple(c) for row in manifest["rows"] for c in row["cells"]]
    return img, boxes


@pytest.mark.parametrize("name", sorted(EXPECTED))
def test_matches_the_typescript_gate(name: str) -> None:
    if not (REF / f"{name}-S.png").exists():
        pytest.skip(f"reference sheets not present at {REF}")
    want_factor, want_conf, want_purity = EXPECTED[name]
    img, boxes = _load(name)
    r = detect_pixel_grid(img, boxes)

    top = max(r.scores, key=lambda s: s[1])
    assert top[0] == want_factor, f"{name}: best factor drifted"
    assert top[1] * 100 == pytest.approx(want_conf, abs=0.15), f"{name}: confidence drifted"
    assert r.cell_purity * 100 == pytest.approx(want_purity, abs=0.1), f"{name}: cell purity drifted"

    # The verdict is the thing anyone actually acts on.
    assert not r.gridded
    assert "NOT PIXEL ART" in r.verdict


@pytest.mark.parametrize("name", sorted(EXPECTED))
def test_flatness_measures_art_not_empty_space(name: str) -> None:
    """The divergence, pinned so it cannot silently revert.

    Counting transparent pairs put the jester at 34% where its ART is 1.7%, and
    `NATIVE_FLAT_SHARE` is a 0.55 gate — a sparse sheet could have crossed it on
    empty space alone.
    """
    if not (REF / f"{name}-S.png").exists():
        pytest.skip(f"reference sheets not present at {REF}")
    img, boxes = _load(name)
    r = detect_pixel_grid(img, boxes)
    assert r.flat_share < 0.10, f"{name}: flatness is counting the empty field again"


#: The RAW frog sheet, before matting — the case the matte exists for.
RAW_FROG = Path(
    os.environ.get(
        "SPRITEFORGE_RAW_FROG",
        "/tmp/claude-1000/-home-lazycat-github-projects-sun/"
        "4d8618f5-3841-4241-a83f-1e43a1bb7365/scratchpad/inbox-stash/frog-ORIGINAL.png",
    )
)


def test_matte_matches_the_typescript() -> None:
    """TS reported: bg #fdfdfd (100% of the border) — keyed 73.2%, 1267 pockets."""
    if not RAW_FROG.exists():
        pytest.skip(f"raw sheet not present at {RAW_FROG}")
    img = np.asarray(Image.open(RAW_FROG).convert("RGBA"))
    _, rep = matte(img)
    assert rgb_hex(rep.bg) == "#fdfdfd"
    assert rep.bg_confidence * 100 == pytest.approx(100, abs=0.5)
    assert rep.keyed_pct * 100 == pytest.approx(73.2, abs=0.1)
    assert len(rep.enclosed) == 1267
    assert rep.failures == []


#: Cell counts the TypeScript slicer reported, per clip row.
SHAPES = {"jester": [4, 4, 6, 2, 4], "beaver": [4, 4, 4, 4], "frog": [5, 5, 5, 5]}


@pytest.mark.parametrize("name", sorted(SHAPES))
def test_slicer_reproduces_every_cell_rect(name: str) -> None:
    """Not just the same SHAPE — the same rects, to the pixel.

    Cell counts can match while every rect is a few pixels off, which then moves
    every downstream number. All 56 rects across the three sheets are identical.
    """
    if not (REF / f"{name}-S.png").exists():
        pytest.skip(f"reference sheets not present at {REF}")
    img = np.asarray(Image.open(REF / f"{name}-S.png").convert("RGBA"))
    rows = slice_sheet_detail(img[..., 3]).rows
    assert [len(r.cells) for r in rows] == SHAPES[name]

    manifest = json.loads((REF / f"{name}-S.json").read_text())
    want = [tuple(c) for row in manifest["rows"] for c in row["cells"]]
    got = [c for r in rows for c in r.cells]
    assert got == want, f"{name}: cell rects drifted from the TypeScript"


CLIPS = {
    "jester": ["idle", "attack", "walk", "stumble", "death"],
    "beaver": ["idle", "walk", "attack", "death"],
    "frog": ["idle", "walk", "attack", "death"],
}


@pytest.mark.parametrize("name", sorted(CLIPS))
def test_shipped_sheets_have_clean_gutters(name: str) -> None:
    """The floor the gate's threshold is set against: exactly zero, on real art."""
    if not (REF / f"{name}-S.png").exists():
        pytest.skip(f"reference sheets not present at {REF}")
    img = np.asarray(Image.open(REF / f"{name}-S.png").convert("RGBA"))
    d = slice_sheet_detail(img[..., 3])
    r = gutter_report(d.mask, d.rows, CLIPS[name])
    assert r.failures == [], r.failures
    worst = max((s.pct for row in r.rows for s in row.seams), default=0.0)
    assert worst == 0.0, f"{name}: worst clean seam is {worst:.3%}, not 0"


def test_it_catches_the_real_laser_bridge() -> None:
    """The defect that shipped: a beam crossing the gap between two attack frames.

    It merged them, so the pipeline saw 4 cells where the pitch expects 5 — which
    is precisely the situation where the returned rects cannot be used.
    """
    if not RAW_FROG.exists():
        pytest.skip(f"raw sheet not present at {RAW_FROG}")
    img = np.asarray(Image.open(RAW_FROG).convert("RGBA"))
    matted, _ = matte(img)
    d = slice_sheet_detail(matted[..., 3])
    r = gutter_report(d.mask, d.rows, CLIPS["frog"])
    assert [len(x.cells) for x in d.rows] == [5, 5, 4, 5], "the fixture no longer reproduces the merge"
    assert r.failures, "the real laser bridge was not caught"
    assert any("attack" in f and "MERGED" in f for f in r.failures)
