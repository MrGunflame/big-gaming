use std::fmt::{self, Debug, Display, Formatter};

use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Local;
use tracing::field::{Field, Visit};
use tracing::metadata::LevelFilter;
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::set_global_default;
use tracing::{Event, Metadata, Subscriber};

static SPAN_ID: AtomicU64 = AtomicU64::new(1);

pub struct Logger {}

impl Logger {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(self) {
        let _ = set_global_default(self);
    }

    fn log<T>(&self, level: LevelFilter, content: T)
    where
        T: Display,
    {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");

        let level = match level {
            LevelFilter::ERROR => "ERROR",
            LevelFilter::WARN => "WARN",
            LevelFilter::INFO => "INFO",
            LevelFilter::DEBUG => "DEBUG",
            LevelFilter::TRACE => "TRACE",
            LevelFilter::OFF => unreachable!(),
        };

        println!("[{}] [{}] {}", now, level, content);
    }
}

impl Subscriber for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let id = SPAN_ID.fetch_add(1, Ordering::Relaxed);
        assert!(id != 0);

        unsafe { Id::from_non_zero_u64(NonZeroU64::new_unchecked(id)) }
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut visitor = Visitor::new();
        values.record(&mut visitor);

        self.log(LevelFilter::INFO, visitor);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event<'_>) {}

    fn enter(&self, span: &Id) {}

    fn exit(&self, span: &Id) {}
}

struct Visitor {
    string: String,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            string: String::with_capacity(32),
        }
    }
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.string += &format!("{} = {:?}", field, value);
    }
}

impl Display for Visitor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.string)
    }
}
