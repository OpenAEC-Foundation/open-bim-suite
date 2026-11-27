//! `O3V_FRAME_LOG=1`: once a second, on stderr, where the time of the interface went: how often
//! and how long the app answered a message (`update`), built its widgets (`view`), handed the 3D
//! scene to the graphics card (`prepare`) and drew a 2D canvas (`canvas`), and the message that
//! took longest. Without the variable it costs one read of a flag per call.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use web_time::Instant;

/// What is timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Update,
    View,
    Prepare,
    Canvas,
}

const PHASES: [(Phase, &str); 4] = [(Phase::Update, "update"), (Phase::View, "view"), (Phase::Prepare, "prepare"), (Phase::Canvas, "canvas")];

#[derive(Default)]
struct Sum {
    count: AtomicU64,
    micros: AtomicU64,
    worst: AtomicU64,
}

struct Log {
    sums: [Sum; 4],
    canvases: Mutex<std::collections::BTreeMap<&'static str, (u64, u64)>>,
    since: Mutex<(Instant, String)>,
}

/// Whether the log is on.
pub fn on() -> bool {
    static ON: OnceLock<AtomicBool> = OnceLock::new();
    ON.get_or_init(|| AtomicBool::new(std::env::var_os("O3V_FRAME_LOG").is_some())).load(Ordering::Relaxed)
}

/// `O3V_FRAME_LOG=spin`: the view moves by itself on every frame, to and fro, so that the log
/// shows how many frames a second the window reaches with nobody at the pointer.
pub fn spin() -> bool {
    static SPIN: OnceLock<bool> = OnceLock::new();
    *SPIN.get_or_init(|| std::env::var("O3V_FRAME_LOG").is_ok_and(|v| v == "spin" || v == "zoom"))
}

/// `O3V_PLAIN=1`: a drawing is made again at every step of a zoom, as it was before drawings
/// were kept while they move, to measure what keeping them gains.
pub fn plain() -> bool {
    static PLAIN: OnceLock<bool> = OnceLock::new();
    *PLAIN.get_or_init(|| std::env::var_os("O3V_PLAIN").is_some())
}

/// The step of a view that moves by itself, by the frame: a pan of some pixels (`spin`) or a
/// zoom (`zoom`), a second one way and a second back.
pub fn spin_step() -> (bool, f32) {
    static N: AtomicU64 = AtomicU64::new(0);
    static START: OnceLock<Instant> = OnceLock::new();
    N.fetch_add(1, Ordering::Relaxed);
    let way = if START.get_or_init(Instant::now).elapsed().as_secs() % 2 == 0 { 1.0 } else { -1.0 };
    (std::env::var("O3V_FRAME_LOG").is_ok_and(|v| v == "zoom"), way)
}

fn log() -> &'static Log {
    static LOG: OnceLock<Log> = OnceLock::new();
    LOG.get_or_init(|| Log { sums: Default::default(), canvases: Mutex::default(), since: Mutex::new((Instant::now(), String::new())) })
}

/// Runs `f`, counting its time under `phase` when the log is on; `what` names it if it turns out
/// the slowest of its second.
pub fn time<T>(phase: Phase, what: impl FnOnce() -> String, f: impl FnOnce() -> T) -> T {
    if !on() {
        return f();
    }
    let start = Instant::now();
    let out = f();
    let micros = start.elapsed().as_micros() as u64;
    let log = log();
    let sum = &log.sums[phase as usize];
    sum.count.fetch_add(1, Ordering::Relaxed);
    sum.micros.fetch_add(micros, Ordering::Relaxed);
    let worst = sum.worst.fetch_max(micros, Ordering::Relaxed) < micros;
    let mut since = log.since.lock().unwrap_or_else(|e| e.into_inner());
    if worst && phase == Phase::Update {
        since.1 = what().chars().take(70).collect();
    }
    if since.0.elapsed().as_secs_f64() >= 1.0 {
        let mut line = String::new();
        for (phase, name) in PHASES {
            let sum = &log.sums[phase as usize];
            let (n, total, worst) = (sum.count.swap(0, Ordering::Relaxed), sum.micros.swap(0, Ordering::Relaxed), sum.worst.swap(0, Ordering::Relaxed));
            if n > 0 {
                line += &format!("{name} {n} x {:.2} ms (worst {:.1}, sum {:.0})  ", total as f64 / n as f64 / 1000.0, worst as f64 / 1000.0, total as f64 / 1000.0);
            }
        }
        for (name, (n, micros)) in std::mem::take(&mut *log.canvases.lock().unwrap_or_else(|e| e.into_inner())) {
            line += &format!("{name} {n} x {:.2} ms  ", micros as f64 / n as f64 / 1000.0);
        }
        eprintln!("frame log: {line}slowest message: {}", since.1);
        *since = (Instant::now(), String::new());
    }
    out
}

/// Times the drawing of the canvas `name` until it is dropped, when the log is on.
pub struct Drawing(Option<(&'static str, Instant)>);

pub fn canvas(name: &'static str) -> Drawing {
    Drawing(on().then(|| (name, Instant::now())))
}

impl Drop for Drawing {
    fn drop(&mut self) {
        if let Some((name, start)) = self.0 {
            let micros = start.elapsed().as_micros() as u64;
            let mut canvases = log().canvases.lock().unwrap_or_else(|e| e.into_inner());
            let sum = canvases.entry(name).or_default();
            *sum = (sum.0 + 1, sum.1 + micros);
        }
    }
}
