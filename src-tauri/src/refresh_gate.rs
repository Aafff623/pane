//! Per-card refresh single-flight gate.
//!
//! Three callers can hit the same provider at once: the card's own ⟳, the
//! periodic full scan, and the backoff retry. The global usage lock only
//! serializes whole cycles — a second cycle still waits and then re-fetches
//! every provider. This gate keys on the card id: the first caller runs the
//! fetch, everyone else joins the in-flight result.
//!
//! Semantics follow cockpit-tools' quota scheduler (CC BY-NC-SA 4.0); the
//! implementation is original. A leader owns the entry until its task ends;
//! waiters get the same value or error. A 120 s timeout and a task-panic
//! both fan out an error instead of leaving a spinner forever.

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Manual,
    Background,
}

/// Waiters beyond this are refused instead of growing the entry without
/// bound; a healthy UI has one or two.
const MAX_WAITERS: usize = 32;

/// Leader budget. A fetch that cannot finish in two minutes is treated as
/// failed; the task is aborted so it cannot keep racing a retry.
const DEFAULT_TIMEOUT_MS: u64 = 120_000;
/// Waiters must outlast the leader, otherwise a slow-but-successful fetch
/// would report a false timeout.
const DEFAULT_PATIENCE_MS: u64 = 130_000;

static TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_TIMEOUT_MS);
static PATIENCE_MS: AtomicU64 = AtomicU64::new(DEFAULT_PATIENCE_MS);

type BoxedResult = Result<Box<dyn Any + Send>, String>;

struct Entry {
    waiters: Vec<oneshot::Sender<BoxedResult>>,
    generation: u64,
    kind: Kind,
}

fn gate() -> &'static Mutex<HashMap<String, Entry>> {
    static GATE: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn timeout_ms() -> u64 {
    TIMEOUT_MS.load(Ordering::Relaxed)
}

fn patience_ms() -> u64 {
    PATIENCE_MS.load(Ordering::Relaxed)
}

fn fan_out<T: Clone + Send + 'static>(
    id: &str,
    generation: u64,
    result: &Result<T, String>,
) {
    let waiters = {
        let mut map = gate().lock().unwrap_or_else(|e| e.into_inner());
        match map.get(id) {
            Some(entry) if entry.generation == generation => {
                map.remove(id).map(|entry| entry.waiters).unwrap_or_default()
            }
            _ => return,
        }
    };
    for waiter in waiters {
        let payload: BoxedResult = match result {
            Ok(value) => Ok(Box::new(value.clone())),
            Err(error) => Err(error.clone()),
        };
        let _ = waiter.send(payload);
    }
}

/// Run `op` at most once per card id at a time; concurrent callers share the
/// leader's result. `T` must be cloneable because every waiter gets a copy.
pub async fn run<T, F>(id: &str, kind: Kind, op: F) -> Result<T, String>
where
    T: Clone + Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    {
        let mut map = gate().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(id) {
            if entry.waiters.len() >= MAX_WAITERS {
                return Err(format!("too many waiters for {id}"));
            }
            eprintln!(
                "[pane] refresh gate: {id} joined an in-flight {:?} fetch ({} waiter(s))",
                entry.kind,
                entry.waiters.len() + 1
            );
            entry.waiters.push(tx);
        } else {
            let generation = next_generation();
            map.insert(id.to_string(), Entry { waiters: vec![tx], generation, kind });
            let owned_id = id.to_string();
            tokio::spawn(async move {
                let mut handle = tokio::spawn(op);
                let result = match tokio::time::timeout(
                    Duration::from_millis(timeout_ms()),
                    &mut handle,
                )
                .await
                {
                    Ok(Ok(value)) => Ok(value),
                    Ok(Err(join)) if join.is_panic() => {
                        Err("refresh task panicked".to_string())
                    }
                    Ok(Err(_)) => Err("refresh task cancelled".to_string()),
                    Err(_) => {
                        handle.abort();
                        Err("refresh timed out".to_string())
                    }
                };
                fan_out(&owned_id, generation, &result);
            });
        }
    }

    match tokio::time::timeout(Duration::from_millis(patience_ms()), rx).await {
        Ok(Ok(Ok(boxed))) => boxed
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| "refresh result type mismatch".to_string()),
        Ok(Ok(Err(error))) => Err(error),
        Ok(Err(_)) => Err("refresh gate dropped the waiter".to_string()),
        Err(_) => Err("refresh wait timed out".to_string()),
    }
}

/// Test seams: shorten both budgets. Value 0 restores the defaults.
#[cfg(test)]
pub fn set_timeouts_for_test(timeout: u64, patience: u64) {
    TIMEOUT_MS.store(if timeout == 0 { DEFAULT_TIMEOUT_MS } else { timeout }, Ordering::Relaxed);
    PATIENCE_MS.store(if patience == 0 { DEFAULT_PATIENCE_MS } else { patience }, Ordering::Relaxed);
}

#[cfg(test)]
pub fn in_flight_for_test() -> usize {
    gate().lock().unwrap_or_else(|e| e.into_inner()).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // One sequential test: the timeout budget is process-global, so parallel
    // tests would race each other's `set_timeouts_for_test`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn gate_semantics() {
        // 10 callers, one network call.
        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for i in 0..10 {
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                let kind = if i == 0 { Kind::Manual } else { Kind::Background };
                run("pane-gate-shared", kind, async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    7u32
                })
                .await
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap(), Ok(7));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(in_flight_for_test(), 0);

        // Distinct ids run independently.
        let mut handles = Vec::new();
        for id in ["pane-gate-a", "pane-gate-b"] {
            handles.push(tokio::spawn(async move {
                run(id, Kind::Background, async move { 1u32 }).await
            }));
        }
        for handle in handles {
            assert_eq!(handle.await.unwrap(), Ok(1));
        }

        // A panicking leader fails every waiter and leaves no entry.
        set_timeouts_for_test(2_000, 2_500);
        let first = tokio::spawn(async {
            run::<u32, _>("pane-gate-panic", Kind::Manual, async {
                tokio::time::sleep(Duration::from_millis(5)).await;
                panic!("boom");
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        let second = tokio::spawn(async {
            run::<u32, _>("pane-gate-panic", Kind::Background, async { 99u32 }).await
        });
        let (a, b) = (first.await.unwrap(), second.await.unwrap());
        assert!(a.is_err() && b.is_err(), "waiters must get an error, not hang: {a:?} {b:?}");
        assert_eq!(in_flight_for_test(), 0);

        // A leader that overruns is aborted and reports a timeout.
        set_timeouts_for_test(30, 500);
        let result = run("pane-gate-timeout", Kind::Manual, async {
            tokio::time::sleep(Duration::from_millis(2_000)).await;
            1u32
        })
        .await;
        assert!(result.is_err());
        assert_eq!(in_flight_for_test(), 0);
        set_timeouts_for_test(0, 0);
    }
}
