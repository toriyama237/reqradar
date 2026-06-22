//! Unique identifiers for captured exchanges.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};

/// Generates unique, time-sortable ids of the form `<unix_millis>-<seq>`.
///
/// Uniqueness does **not** depend on the timestamp. `seq` is a process-global
/// monotonic counter that never resets, so two requests landing in the same
/// millisecond still receive distinct ids (`...674-000001`, `...674-000002`).
/// The millis prefix only exists to make ids human-readable and roughly
/// lexicographically sortable; the counter is what guarantees no collisions,
/// even under the concurrent, one-task-per-connection load the proxy generates.
pub struct IdGen {
    seq: AtomicU64,
}

impl IdGen {
    pub fn new() -> Self {
        Self {
            seq: AtomicU64::new(1),
        }
    }

    /// Mint the next id. `now` is only used for the readable prefix.
    pub fn next(&self, now: DateTime<Utc>) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("{}-{:06}", now.timestamp_millis(), seq)
    }
}

impl Default for IdGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    /// The pathological case: every id minted with the *same* timestamp must
    /// still be unique. This is what protects replay from ever resolving the
    /// wrong request when a fast local backend bursts within one millisecond.
    #[test]
    fn ids_are_unique_within_one_millisecond() {
        let gen = IdGen::new();
        let now = Utc::now();
        let n = 100_000;

        let ids: HashSet<String> = (0..n).map(|_| gen.next(now)).collect();

        assert_eq!(ids.len(), n, "ids collided within a single millisecond");
    }

    /// Same invariant, but minted concurrently from many threads sharing one
    /// generator — mirroring the proxy spawning a task per connection.
    #[test]
    fn ids_are_unique_under_concurrency() {
        let gen = Arc::new(IdGen::new());
        let now = Utc::now();
        let threads = 16;
        let per_thread = 10_000;

        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let gen = gen.clone();
                thread::spawn(move || (0..per_thread).map(|_| gen.next(now)).collect::<Vec<_>>())
            })
            .collect();

        let mut all = HashSet::new();
        for handle in handles {
            for id in handle.join().unwrap() {
                assert!(all.insert(id), "duplicate id minted across threads");
            }
        }

        assert_eq!(all.len(), threads * per_thread);
    }
}
