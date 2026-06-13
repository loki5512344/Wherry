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
