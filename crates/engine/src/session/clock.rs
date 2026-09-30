//! Injectable clocks. The personal-learning windows (3 s context keep, 8 s chain pause, 10 s pick-pair gap) read the steady clock and the date/time mode reads the local wall clock; tests replace both.

use std::time::Instant;

use crate::local::date_time::{current_local_date_time, LocalDateTime};

pub struct Clock {
    pub steady: Box<dyn Fn() -> Instant + Send>,
    pub local: Box<dyn Fn() -> LocalDateTime + Send>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            steady: Box::new(Instant::now),
            local: Box::new(current_local_date_time),
        }
    }
}
