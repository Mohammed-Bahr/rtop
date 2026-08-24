use std::collections::VecDeque;

/// Fixed-capacity rolling history of numeric samples.
///
/// Oldest samples are dropped once `capacity` is reached, so memory use is
/// strictly bounded regardless of how long the app runs.
#[derive(Debug, Clone)]
pub struct History {
    data: VecDeque<f64>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn last(&self) -> Option<f64> {
        self.data.back().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().copied()
    }

    /// Samples clamped to `0..=max`, suitable for ratatui's `Sparkline`
    /// which renders `u64` values.
    pub fn sparkline(&self, max: f64) -> Vec<u64> {
        self.data
            .iter()
            .map(|v| v.clamp(0.0, max).max(0.0) as u64)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_growth() {
        let mut h = History::new(3);
        for i in 0..10 {
            h.push(i as f64);
        }
        assert_eq!(h.iter().count(), 3);
        let vals: Vec<f64> = h.iter().collect();
        assert_eq!(vals, vec![7.0, 8.0, 9.0]);
        assert_eq!(h.last(), Some(9.0));
    }

    #[test]
    fn zero_capacity_is_safe() {
        let mut h = History::new(0);
        h.push(1.0);
        h.push(2.0);
        assert_eq!(h.iter().count(), 1);
    }

    #[test]
    fn sparkline_clamps() {
        let mut h = History::new(4);
        for v in [-5.0, 50.0, 120.0] {
            h.push(v);
        }
        assert_eq!(h.sparkline(100.0), vec![0, 50, 100]);
    }
}
