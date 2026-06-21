pub trait Clock {
    fn now_utc(&self) -> String;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

#[derive(Debug, Clone)]
pub struct FixedClock {
    now: String,
}

impl FixedClock {
    pub fn new(now: impl Into<String>) -> Self {
        Self { now: now.into() }
    }
}

impl Clock for FixedClock {
    fn now_utc(&self) -> String {
        self.now.clone()
    }
}
