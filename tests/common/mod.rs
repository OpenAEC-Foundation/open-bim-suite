//! Damaged files for the robustness tests: the mutations, and a way to run a reader on each and
//! learn where it panicked. Shared by `tests/malformed_input.rs` (the samples in `test-models/`)
//! and by the reader modules (`loaders::damage`, in `src/loaders/mod.rs`), which feed it the files
//! their own tests write.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

/// Random mutations per file.
const ROUNDS: u64 = 12;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// The damaged variants of a file, each with a name. Deterministic, so a failure repeats.
pub fn mutations(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    mutations_n(data, ROUNDS)
}

/// As [`mutations`], with `rounds` random rounds: the small files the reader tests write need many.
pub fn mutations_n(data: &[u8], rounds: u64) -> Vec<(String, Vec<u8>)> {
    let len = data.len();
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for cut in [0, 1, 4, 16, len / 4, len / 2, len * 3 / 4, len.saturating_sub(1)] {
        if cut < len {
            out.push((format!("truncated at {cut}"), data[..cut].to_vec()));
        }
    }
    if len > 0 {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ len as u64);
        for round in 0..rounds {
            // Flip bytes anywhere
            let mut flipped = data.to_vec();
            for _ in 0..(1 + rng.below(16)) {
                let at = rng.below(len);
                flipped[at] = rng.next() as u8;
            }
            out.push((format!("random bytes {round}"), flipped));
            // Damage the header, where sizes and counts usually are
            let mut head = data.to_vec();
            for _ in 0..(1 + rng.below(8)) {
                let at = rng.below(len.min(256));
                head[at] = rng.next() as u8;
            }
            out.push((format!("header {round}"), head));
        }
        // A window of zeros, and a window of 0xFF (sizes that claim too much)
        let at = len / 3;
        let end = (at + 64).min(len);
        let mut zeros = data.to_vec();
        zeros[at..end].fill(0);
        out.push(("zero window".into(), zeros));
        let mut ones = data.to_vec();
        ones[at..end].fill(0xFF);
        out.push(("0xFF window".into(), ones));
    }
    out
}

thread_local! {
    /// Where the last panic of this thread was, while `catch` runs.
    static PLACE: RefCell<Option<String>> = const { RefCell::new(None) };
    static CATCHING: Cell<bool> = const { Cell::new(false) };
}

/// Installs a hook that records the place of a panic inside `catch` (per thread, so tests may run
/// in parallel) and leaves every other panic to the hook that was there.
fn install_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if CATCHING.with(Cell::get) {
                let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
                let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
                // the first panic of a run is the cause; later ones come from unwinding
                PLACE.with(|p| {
                    let mut p = p.borrow_mut();
                    if p.is_none() {
                        *p = Some(format!("{at}: {message}"));
                    }
                });
            } else {
                previous(info);
            }
        }));
    });
}

/// Runs `work`; a panic comes back as `Err("file:line: message")`.
pub fn catch<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    catch_noting(work).0
}

/// As [`catch`], and also the place of a panic that `work` caught itself (a safety net inside it).
pub fn catch_noting<T>(work: impl FnOnce() -> T) -> (Result<T, String>, Option<String>) {
    install_hook();
    PLACE.with(|p| *p.borrow_mut() = None);
    CATCHING.with(|c| c.set(true));
    let result = panic::catch_unwind(AssertUnwindSafe(work));
    CATCHING.with(|c| c.set(false));
    let place = PLACE.with(|p| p.borrow_mut().take());
    (result.map_err(|_| place.clone().unwrap_or_else(|| "unknown".into())), place)
}

/// Runs `read` on every damaged variant of `seed`; a panic is a failure, listed by place.
/// `read` is the reader below any safety net, and gets bytes it may reject with an error.
pub fn assert_robust(label: &str, seed: &[u8], read: impl Fn(&[u8])) {
    let mut found: BTreeMap<String, (String, u32)> = BTreeMap::new();
    let variants = mutations_n(seed, 400);
    let tried = variants.len();
    for (what, bytes) in variants {
        if let Err(place) = catch(|| read(&bytes)) {
            let entry = found.entry(place).or_insert_with(|| (what, 0));
            entry.1 += 1;
        }
    }
    if !found.is_empty() {
        let mut report = String::new();
        for (place, (first, times)) in &found {
            report.push_str(&format!("  {place}\n      first: {first} ({times} of {tried})\n"));
        }
        panic!("{label}: damaged files made the reader panic:\n{report}");
    }
}
