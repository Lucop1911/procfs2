use std::time::{Duration, Instant};

/// Stores the previous snapshot in a sampling loop.
#[derive(Debug)]
pub struct Sampler<T> {
    last: Option<(T, Instant)>,
}

impl<T> Sampler<T> {
    /// Creates an empty sampler.
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Replaces the previous snapshot and returns it with elapsed time.
    ///
    /// The first update returns `None`, because there is no earlier snapshot
    /// to compare with.
    pub fn update(&mut self, snapshot: T) -> Option<(T, Duration)> {
        let now = Instant::now();
        self.last
            .replace((snapshot, now))
            .map(|(previous, timestamp)| (previous, now.duration_since(timestamp)))
    }
}

impl<T> Default for Sampler<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Sampler;

    #[test]
    fn first_update_has_no_previous_snapshot() {
        let mut sampler = Sampler::new();
        assert!(sampler.update(1).is_none());
    }

    #[test]
    fn later_update_returns_previous_snapshot() {
        let mut sampler = Sampler::new();
        sampler.update(1);
        let (previous, elapsed) = match sampler.update(2) {
            Some(value) => value,
            None => panic!("second update should have a previous snapshot"),
        };
        assert_eq!(previous, 1);
        assert!(elapsed <= std::time::Duration::from_secs(1));
    }
}
