use std::time::{Duration, Instant};

/// Throttle прогресс-событий — максимум 10 в секунду.
/// Пользователь разницы не увидит, а тысяч emit'ов не будет.
pub struct ProgressThrottle {
    last_emit: Instant,
    interval: Duration,
}

impl Default for ProgressThrottle {
    fn default() -> Self {
        Self {
            last_emit: Instant::now() - Duration::from_secs(1),
            interval: Duration::from_millis(100), // 10 fps
        }
    }
}

impl ProgressThrottle {
    pub fn should_emit(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_emit) >= self.interval {
            self.last_emit = now;
            true
        } else {
            false
        }
    }

    /// Всегда emit при завершении/ошибке
    pub fn force(&mut self) -> bool {
        self.last_emit = Instant::now() - self.interval;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_call_emits() {
        let mut throttle = ProgressThrottle::default();
        assert!(throttle.should_emit());
    }

    #[test]
    fn test_too_soon_does_not_emit() {
        let mut throttle = ProgressThrottle::default();
        throttle.should_emit();
        assert!(!throttle.should_emit());
    }

    #[test]
    fn test_force_resets() {
        let mut throttle = ProgressThrottle::default();
        throttle.should_emit();
        assert!(throttle.force());
        assert!(throttle.should_emit());
    }

    #[test]
    fn test_interval_elapsed_emits() {
        let mut throttle = ProgressThrottle {
            last_emit: Instant::now() - Duration::from_millis(200),
            interval: Duration::from_millis(100),
        };
        assert!(throttle.should_emit());
    }
}
