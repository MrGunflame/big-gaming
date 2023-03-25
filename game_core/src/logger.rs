use std::fmt::{self, Debug, Display, Formatter};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Local;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Record};
use tracing::subscriber::set_global_default;
use tracing::{Event, Id, Level, Metadata, Subscriber};

pub fn init() {
    Logger::new().init();
}

#[derive(Debug)]
pub struct Logger {
    id: AtomicU64,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            id: AtomicU64::new(1),
        }
    }

    pub fn init(self) {
        let _ = set_global_default(self);
    }
}

impl Logger {
    fn log<T>(&self, level: Level, content: T)
    where
        T: Display,
    {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");

        let level = match level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        println!("[{}] [{}] {}", now, level, content);
    }
}

impl Subscriber for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        assert!(id != 0);

        unsafe { Id::from_non_zero_u64(NonZeroU64::new_unchecked(id)) }
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut visitor = Visitor::new();
        values.record(&mut visitor);

        self.log(Level::INFO, visitor);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = Visitor::new();
        event.record(&mut visitor);

        self.log(*event.metadata().level(), visitor);
    }

    fn enter(&self, span: &Id) {
        self.log(Level::INFO, format!("--> {}", span.into_u64()));
    }

    fn exit(&self, span: &Id) {
        self.log(Level::INFO, format!("<-- {}", span.into_u64()));
    }
}

struct Visitor {
    buf: String,
}

impl Visitor {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(32),
        }
    }
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.buf += &format!("{} = {:?}", field, value);
    }
}

impl Display for Visitor {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.buf, f)
    }
}
