//! Smoke test: snaps a synthetic sprite through the real batch pipeline
//! (`process_batch_with_reporter`, the native entry point behind the CLI and
//! the web worker) and compares the output against the expected result:
//! a byte-deterministic run whose four colour regions stay recognisable,
//! whose colour set collapses towards the requested `k_colors`, and whose
//! digest matches the recorded one.

use image::{ImageBuffer, Rgba, RgbaImage};
use spritefusion_pixel_snapper::{process_batch_with_reporter, BatchConfig};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const K_COLORS: usize = 4;

/// FNV-1a hex over the snapped PNG bytes (recorded from the pinned input).
const EXPECTED_OUTPUT_DIGEST: &str = "2ce914fcab42c984";

const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
const GREEN: Rgba<u8> = Rgba([0, 255, 0, 255]);
const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

/// A 64x64 four-quadrant sprite with a 2px black cross: hard edges the
/// pixel-grid/segmentation logic must lock onto, four saturated colours the
/// k-means step must recover.
fn synthetic_sprite() -> RgbaImage {
    ImageBuffer::from_fn(64, 64, |x, y| {
        if (31..33).contains(&x) || (31..33).contains(&y) {
            return BLACK;
        }
        match (x < 32, y < 32) {
            (true, true) => RED,
            (false, true) => GREEN,
            (true, false) => BLUE,
            (false, false) => WHITE,
        }
    })
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// One full batch run on a fresh tree; returns the snapped PNG bytes.
fn snap_once(tag: &str) -> Vec<u8> {
    let work = std::env::temp_dir().join(format!("snapper-smoke-{tag}-{}", std::process::id()));
    let input_dir = work.join("in");
    let output_dir = work.join("out");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&input_dir).unwrap();
    synthetic_sprite()
        .save(input_dir.join("sprite.png"))
        .expect("write synthetic input");

    let config = BatchConfig {
        input_dir: input_dir.clone(),
        output_dir: output_dir.clone(),
        k_colors: K_COLORS,
        pixel_size_override: None,
        palette: None,
    };
    let mut finished = AtomicUsize::new(0);
    process_batch_with_reporter(&config, |ev| {
        if let spritefusion_pixel_snapper::BatchEvent::Finished { .. } = ev {
            finished.fetch_add(1, Ordering::Relaxed);
        }
    })
    .expect("batch snap failed");
    assert_eq!(
        finished.load(Ordering::Relaxed),
        1,
        "exactly one image must be processed"
    );

    let outs: Vec<PathBuf> = fs::read_dir(&output_dir)
        .expect("output dir")
        .filter_map(|e| {
            let p = e.expect("entry").path();
            (p.extension().map(|x| x == "png").unwrap_or(false)).then_some(p)
        })
        .collect();
    assert_eq!(outs.len(), 1, "expected exactly one snapped PNG");
    let bytes = fs::read(&outs[0]).expect("read snapped png");
    let _ = fs::remove_dir_all(&work);
    bytes
}

/// Mean colour of a relative window of the snapped image.
fn mean_region(img: &RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32) -> [f32; 3] {
    let w = img.width();
    let h = img.height();
    let (xa, xb) = ((x0 * w as f32) as u32, (x1 * w as f32).max(1.0) as u32);
    let (ya, yb) = ((y0 * h as f32) as u32, (y1 * h as f32).max(1.0) as u32);
    let (xa, xb) = (xa.min(w - 1), xb.min(w).max(xa + 1));
    let (ya, yb) = (ya.min(h - 1), yb.min(h).max(ya + 1));
    let (mut r, mut g, mut b, mut n) = (0f64, 0f64, 0f64, 0f64);
    for y in ya..yb {
        for x in xa..xb {
            let px = img.get_pixel(x, y);
            r += f64::from(px[0]);
            g += f64::from(px[1]);
            b += f64::from(px[2]);
            n += 1.0;
        }
    }
    [
        r as f32 / n as f32,
        g as f32 / n as f32,
        b as f32 / n as f32,
    ]
}

#[test]
fn snapping_a_synthetic_sprite_is_deterministic_and_matches_expectations() {
    let a = snap_once("a");
    let b = snap_once("b");

    // Same input, same config => byte-identical output (seeded k-means).
    assert_eq!(a, b, "two snaps of the identical input diverged");

    let img = image::load_from_memory(&a)
        .expect("snapped output decodes")
        .to_rgba8();
    assert!(
        img.width() >= 16 && img.height() >= 16,
        "output collapsed: {}x{}",
        img.width(),
        img.height()
    );

    // The colour set must collapse from 32-bit RGBA towards k = 4
    // (dithering may pair colours; allow a generous 2*k).
    let colours: HashSet<[u8; 3]> = img.pixels().map(|p| [p[0], p[1], p[2]]).collect();
    assert!(
        colours.len() <= 2 * K_COLORS,
        "colour reduction failed: {} distinct colours",
        colours.len()
    );

    // Each quadrant must still read as its source colour.
    let tl = mean_region(&img, 0.10, 0.10, 0.40, 0.40);
    let tr = mean_region(&img, 0.60, 0.10, 0.90, 0.40);
    let bl = mean_region(&img, 0.10, 0.60, 0.40, 0.90);
    let br = mean_region(&img, 0.60, 0.60, 0.90, 0.90);
    assert!(
        tl[0] > 150.0 && tl[1] < 100.0 && tl[2] < 100.0,
        "top-left lost red: {tl:?}"
    );
    assert!(
        tr[1] > 150.0 && tr[0] < 100.0 && tr[2] < 100.0,
        "top-right lost green: {tr:?}"
    );
    assert!(
        bl[2] > 150.0 && bl[0] < 100.0 && bl[1] < 100.0,
        "bottom-left lost blue: {bl:?}"
    );
    assert!(
        br[0] > 150.0 && br[1] > 150.0 && br[2] > 150.0,
        "bottom-right lost white: {br:?}"
    );

    // Pinned digest of the whole PNG (re-record if intended).
    let digest = format!("{:016x}", fnv1a(&a));
    assert_eq!(digest, EXPECTED_OUTPUT_DIGEST, "snapped output drifted");
}

/// The batch API refuses to overwrite its input directory.
#[test]
fn batch_refuses_in_place_output() {
    let work = std::env::temp_dir().join(format!("snapper-smoke-same-{}", std::process::id()));
    fs::create_dir_all(&work).unwrap();
    let config = BatchConfig {
        input_dir: work.clone(),
        output_dir: work.clone(),
        k_colors: K_COLORS,
        pixel_size_override: None,
        palette: None,
    };
    let err = process_batch_with_reporter(&config, |_| {}).unwrap_err();
    assert!(
        err.to_string().to_lowercase().contains("different"),
        "expected a guard error, got {err}"
    );
    let _ = fs::remove_dir_all(&work);
}
