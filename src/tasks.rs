//! Background tasks: one place to run slow work off the UI thread.
//!
//! # Why this exists
//!
//! Kalam had four different ways of doing background work — a bare
//! `thread::spawn`, `thread::spawn` plus an `async_channel`, relm4's
//! `spawn_command`, and "just do it on the UI thread and hope it is quick".
//! The last one is not a strategy: importing a dictionary froze the window for
//! as long as the parse took, with nothing on screen to say why.
//!
//! This module is A0 step 4. It gives that work a single shape: a worker
//! thread for the slow part, progress reported as it goes, a result delivered
//! back **on the main thread**, and a cancellation flag the work can check.
//!
//! # The rule it enforces
//!
//! GTK widgets and [`crate::notify`] belong to the main thread. A worker must
//! never touch them. So the closure that *does* the work is `Send` and gets no
//! access to the UI, while the closures that report progress and handle the
//! result are **not** `Send` — they are only ever run on the main thread, so
//! they can freely touch widgets and raise toasts.
//!
//! That split is the whole point: the type system stops you handing a widget
//! to a worker thread, which is a bug you otherwise find by crashing.
//!
//! # No tokio
//!
//! `thread::spawn` + `async-channel` + the GLib main loop, per the roadmap.
//! relm4 is already the actor framework; a second runtime would be a second
//! scheduler fighting the first.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Every task currently running, so [`cancel_all`] can reach them.
///
/// A plain `Vec` because there are single digits of these at once; a map would
/// be more code for no measurable gain.
type Registry = Mutex<Vec<(u64, Arc<AtomicBool>)>>;

static RUNNING: OnceLock<Registry> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn registry() -> &'static Registry {
    RUNNING.get_or_init(|| Mutex::new(Vec::new()))
}

/// A lock that survives a panic in another task.
///
/// `Mutex::lock` returns `Err` once a holder has panicked, and the usual
/// `.expect()` would then turn one failed task into a crash on the next one.
/// The registry is a plain list of flags — a poisoned one is still perfectly
/// readable, so take the data and carry on.
fn locked<R>(f: impl FnOnce(&mut Vec<(u64, Arc<AtomicBool>)>) -> R) -> R {
    let mut guard = match registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    f(&mut guard)
}

/// Handed to the worker closure. Its only two jobs are reporting progress and
/// answering "should I stop?".
///
/// Deliberately **not** `Sync`-friendly beyond what it needs: it carries a
/// channel and a flag, and no way to reach the UI.
pub struct Reporter {
    tx: async_channel::Sender<Update>,
    cancelled: Arc<AtomicBool>,
}

/// A progress report from a worker.
#[derive(Debug, Clone)]
pub struct Update {
    /// Units finished so far.
    pub done: usize,
    /// Total units, or `0` when the worker cannot know it up front.
    pub total: usize,
    /// What is happening right now — a file name, a dictionary name.
    pub detail: String,
}

impl Reporter {
    /// Report progress. Cheap and non-blocking; safe to call in a tight loop.
    ///
    /// The send is deliberately ignored on failure: the only way it fails is
    /// the UI side having gone away, and a worker should not care.
    pub fn step(&self, done: usize, total: usize, detail: impl Into<String>) {
        let _ = self.tx.send_blocking(Update {
            done,
            total,
            detail: detail.into(),
        });
    }

    /// Whether the task has been asked to stop.
    ///
    /// Cancellation is cooperative — nothing can safely kill a thread
    /// mid-write — so long-running work must check this between units and
    /// return early when it is true.
    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

/// Run `work` on a worker thread; report progress and the result on the main
/// thread.
///
/// * `work` is `Send` and receives a [`Reporter`]. It must not touch GTK or
///   [`crate::notify`].
/// * `on_progress` runs on the main thread for each [`Reporter::step`].
/// * `on_done` runs on the main thread once, with whatever `work` returned.
///   It runs even when the task was cancelled — the worker decides what a
///   cancelled result looks like, because only it knows how far it got.
///
/// Must be called from the main thread: it attaches a receiver to the GLib
/// main context.
pub fn spawn<T, W, P, D>(work: W, on_progress: P, on_done: D)
where
    T: Send + 'static,
    W: FnOnce(Reporter) -> T + Send + 'static,
    P: Fn(Update) + 'static,
    D: FnOnce(T) + 'static,
{
    // Two channels rather than one enum: it keeps `Reporter` non-generic, so
    // worker code does not have to name the task's result type to report a
    // percentage.
    let (progress_tx, progress_rx) = async_channel::unbounded::<Update>();
    let (result_tx, result_rx) = async_channel::unbounded::<T>();

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let cancelled = Arc::new(AtomicBool::new(false));
    locked(|running| running.push((id, cancelled.clone())));

    std::thread::spawn(move || {
        let reporter = Reporter {
            tx: progress_tx,
            cancelled,
        };
        let out = work(reporter);
        // `reporter` is dropped here, which closes the progress channel and
        // is how the reader below knows to stop waiting for updates.
        let _ = result_tx.send_blocking(out);
    });

    gtk::glib::spawn_future_local(async move {
        // Drain progress to exhaustion *first*, then take the result. One
        // sequential future rather than two concurrent ones, because two would
        // race: the completion toast could land before the last progress
        // update and leave a stale "importing 3 of 5" on screen afterwards.
        while let Ok(update) = progress_rx.recv().await {
            on_progress(update);
        }
        if let Ok(value) = result_rx.recv().await {
            on_done(value);
        }
        locked(|running| running.retain(|(other, _)| *other != id));
    });
}

/// Run `work` on a worker thread, delivering each item it produces to the main
/// thread as it is produced.
///
/// [`spawn`] answers "do this, tell me when it is finished". This answers
/// "keep producing things until you run out". A preloader is the second shape:
/// it decodes twenty covers and each one should appear the moment it is ready,
/// not twenty covers later.
///
/// `work` gets an [`Emit`] as well as a [`Reporter`]. `on_item` runs on the
/// main thread once per emitted value, so it may touch widgets; like `spawn`,
/// it is deliberately not `Send`.
pub fn spawn_stream<T, W, F>(work: W, on_item: F)
where
    T: Send + 'static,
    W: FnOnce(Reporter, Emit<T>) + Send + 'static,
    F: Fn(T) + 'static,
{
    // A stream reports by emitting items, so there is no separate progress
    // channel to listen on. The `Reporter` still exists for `cancelled()`;
    // its sender goes nowhere, which is fine because `step` ignores send
    // failures by design.
    let (progress_tx, progress_rx) = async_channel::unbounded::<Update>();
    drop(progress_rx);
    let (item_tx, item_rx) = async_channel::unbounded::<T>();

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let cancelled = Arc::new(AtomicBool::new(false));
    locked(|running| running.push((id, cancelled.clone())));

    std::thread::spawn(move || {
        let reporter = Reporter {
            tx: progress_tx,
            cancelled,
        };
        work(reporter, Emit { tx: item_tx });
    });

    gtk::glib::spawn_future_local(async move {
        // Ends when the worker drops its `Emit`, which closes the channel.
        while let Ok(item) = item_rx.recv().await {
            on_item(item);
        }
        locked(|running| running.retain(|(other, _)| *other != id));
    });
}

/// The worker half of [`spawn_stream`]: hands finished items back one at a
/// time.
pub struct Emit<T> {
    tx: async_channel::Sender<T>,
}

impl<T> Emit<T> {
    /// Deliver one finished item to the main thread.
    ///
    /// Returns `false` once the UI side has gone away, which is a worker's cue
    /// to stop early — a preloader has no reason to keep decoding for a page
    /// that has been closed.
    pub fn send(&self, item: T) -> bool {
        self.tx.send_blocking(item).is_ok()
    }
}

/// Ask every running task to stop.
///
/// Called when the window is closing. Cancellation is cooperative, so this
/// only sets flags — a task that never checks [`Reporter::cancelled`] runs to
/// completion, which is correct for short ones.
pub fn cancel_all() {
    locked(|running| {
        for (_, flag) in running.iter() {
            flag.store(true, Ordering::Relaxed);
        }
    });
}

/// How many tasks are running. Used by the shutdown path and the tests.
pub fn running_count() -> usize {
    locked(|running| running.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a reporter without going through `spawn`, which needs a GLib main
    /// context and therefore cannot run in CI (no display).
    fn reporter() -> (Reporter, async_channel::Receiver<Update>, Arc<AtomicBool>) {
        let (tx, rx) = async_channel::unbounded::<Update>();
        let flag = Arc::new(AtomicBool::new(false));
        (
            Reporter {
                tx,
                cancelled: flag.clone(),
            },
            rx,
            flag,
        )
    }

    #[test]
    fn progress_reaches_the_other_side() {
        let (reporter, rx, _flag) = reporter();
        reporter.step(3, 10, "chapter-3.xhtml");
        let got = rx.try_recv().expect("an update was sent");
        assert_eq!(got.done, 3);
        assert_eq!(got.total, 10);
        assert_eq!(got.detail, "chapter-3.xhtml");
    }

    #[test]
    fn a_worker_sees_the_cancel_flag() {
        let (reporter, _rx, flag) = reporter();
        assert!(!reporter.cancelled(), "starts running");
        flag.store(true, Ordering::Relaxed);
        assert!(reporter.cancelled(), "sees the flag flip");
    }

    #[test]
    fn reporting_after_the_ui_is_gone_is_not_an_error() {
        // The window can close while a worker is mid-loop. Dropping the
        // receiver must not panic the worker or make it bail out early.
        let (reporter, rx, _flag) = reporter();
        drop(rx);
        reporter.step(1, 2, "still going");
        assert!(!reporter.cancelled());
    }

    #[test]
    fn emit_reports_when_the_ui_side_has_gone() {
        // A preloader uses this as its stop signal: once the page is torn
        // down there is nothing to decode for, so `send` must say so rather
        // than fail silently and let the worker grind on.
        let (tx, rx) = async_channel::unbounded::<u8>();
        let emit = Emit { tx };
        assert!(emit.send(1), "delivers while the receiver lives");
        drop(rx);
        assert!(!emit.send(2), "reports the receiver being gone");
    }

    #[test]
    fn cancel_all_flags_every_registered_task() {
        // Uses the real registry, so clean up after itself rather than
        // assuming it starts empty -- tests share the process.
        let before = running_count();
        let a = Arc::new(AtomicBool::new(false));
        let b = Arc::new(AtomicBool::new(false));
        locked(|running| {
            running.push((90_001, a.clone()));
            running.push((90_002, b.clone()));
        });
        assert_eq!(running_count(), before + 2);

        cancel_all();
        assert!(a.load(Ordering::Relaxed));
        assert!(b.load(Ordering::Relaxed));

        locked(|running| running.retain(|(id, _)| *id != 90_001 && *id != 90_002));
        assert_eq!(running_count(), before);
    }

    #[test]
    fn a_poisoned_registry_does_not_take_the_next_task_down() {
        // One task panicking must not turn every later task into a crash.
        let _ = std::panic::catch_unwind(|| {
            locked(|_| panic!("worker exploded while holding the lock"));
        });
        // The lock is poisoned now; this must still work.
        let _ = running_count();
        cancel_all();
    }
}
