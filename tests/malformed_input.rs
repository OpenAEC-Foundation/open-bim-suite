//! Damaged files must give an error, never a panic.
//!
//! Every sample in `test-models/` is read through the same door the app uses
//! (`loaders::load_with`, from bytes, as the browser does), truncated, zero-filled and with random
//! bytes flipped. A loader may return `Err` or an empty model; it may not panic.
//! The mutations are deterministic, so a failure repeats: the report names the file, the mutation
//! and the place of the panic. `load_with` catches a panic and returns it as an error; the test fails
//! only when one gets out, and lists the ones caught, which are still bugs of the reader.
//!
//! Formats with no sample in the repository are covered by tests in their own modules, with the
//! files their tests write (`loaders::damage`).

mod common;

use open_3d_viewer::loaders::{self, Progress, Source};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Samples above this size are left out: the mutations would make the test slow.
const MAX_SAMPLE: u64 = 1_500_000;

fn samples(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            samples(&path, out);
        } else if path.metadata().map(|m| m.len() <= MAX_SAMPLE).unwrap_or(false) {
            out.push(path);
        }
    }
}

#[test]
fn damaged_files_do_not_panic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-models");
    let mut files = Vec::new();
    samples(&root, &mut files);

    // place of the panic -> (first sample and mutation that hit it, how often)
    let mut escaped: BTreeMap<String, (String, u32)> = BTreeMap::new();
    let mut contained: BTreeMap<String, (String, u32)> = BTreeMap::new();
    let mut tried = 0u32;
    let supported = loaders::supported_extensions();
    for file in &files {
        let ext = file.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !supported.contains(&ext.as_str()) {
            continue;
        }
        let Ok(data) = std::fs::read(file) else { continue };
        let name = file.strip_prefix(&root).unwrap_or(file).to_string_lossy().replace('\\', "/");
        let file_name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        for (what, bytes) in common::mutations(&data) {
            tried += 1;
            let source = Source::Bytes { name: file_name.clone(), data: Arc::new(bytes) };
            let (result, place) = common::catch_noting(|| loaders::load_with(&source, &Progress::default()));
            match (result, place) {
                // got out of `load_with`: the safety net failed
                (Err(place), _) => escaped.entry(place).or_insert_with(|| (format!("{name}, {what}"), 0)).1 += 1,
                // a reader panicked and `load_with` turned it into an error: still a bug in the reader
                (Ok(_), Some(place)) => contained.entry(place).or_insert_with(|| (format!("{name}, {what}"), 0)).1 += 1,
                (Ok(_), None) => {}
            }
        }
    }

    eprintln!("{tried} damaged files tried, {} places panic beyond the safety net, {} inside it", escaped.len(), contained.len());
    for (place, (first, times)) in &contained {
        eprintln!("  caught by the safety net: {place}\n      first: {first} ({times} times)");
    }
    if !escaped.is_empty() {
        let mut report = String::new();
        for (place, (first, times)) in &escaped {
            report.push_str(&format!("  {place}\n      first: {first} ({times} times)\n"));
        }
        panic!("damaged files made the loaders panic:\n{report}");
    }
}
