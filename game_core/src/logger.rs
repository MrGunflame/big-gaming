use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::io::IsTerminal;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Local;
use parking_lot::RwLock;
use tracing::field::{Field, Visit};
use tracing::metadata::LevelFilter;
use tracing::span::{Attributes, Record};
use tracing::subscriber::set_global_default;
use tracing::{Event, Id, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub fn init() {
    let layer = tracing_subscriber::registry();
    #[cfg(feature = "tracy")]
    let layer = layer.with(game_tracing::TracyLayer::default());
    let layer = layer.with(Logger::new());

    set_global_default(layer).unwrap();
}

#[derive(Debug)]
pub struct Logger {
    id: AtomicU64,
    spans: RwLock<HashMap<Id, SpanDetails>>,
    is_tty: bool,
    level: LevelFilter,
}

impl Logger {
    pub fn new() -> Self {
        let level = std::env::var("RUST_LOG")
            .map(|e| match e.as_str() {
                "error" | "ERROR" => LevelFilter::ERROR,
                "warn" | "WARN" => LevelFilter::WARN,
                "info" | "INFO" => LevelFilter::INFO,
                "debug" | "DEBUG" => LevelFilter::DEBUG,
                "trace" | "TRACE" => LevelFilter::TRACE,
                "off" | "OFF" => LevelFilter::OFF,
                _ => LevelFilter::INFO,
            })
            .unwrap_or(LevelFilter::INFO);

        let is_tty = std::io::stdout().is_terminal();

        Self {
            id: AtomicU64::new(1),
            spans: RwLock::new(HashMap::new()),
            is_tty,
            level,
        }
    }
}

impl Logger {
    fn log<T>(&self, meta: &SpanDetails, content: T)
    where
        T: Display,
    {
        // Discard if more verbose than max level.
        if meta.level > self.level {
            return;
        }

        let now = Local::now().format("%Y-%m-%d %H:%M:%S:%f");

        let (level, color) = match meta.level {
            Level::ERROR => ("ERROR", Color::RED),
            Level::WARN => ("WARN", Color::YELLOW),
            Level::INFO => ("INFO", Color::GREEN),
            Level::DEBUG => ("DEBUG", Color::LIGHT_GRAY_BOLD),
            Level::TRACE => ("TRACE", Color::LIGHT_GRAY_BOLD),
        };

        if self.is_tty {
            let now = ColorText::new(format!("[{}]", now), Color::LIGHT_GRAY);
            let level = ColorText::new(level, color);
            let name = ColorText::new(&meta.name, Color::LIGHT_GRAY);

            println!("{} {} {} {}", now, level, name, content);
        } else {
            let now = format!("[{}]", now);
            let name = &meta.name;

            println!("{} {} {} {}", now, level, name, content);
        };
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for Logger
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() > self.level {
            return;
        }

        let mut visitor = Visitor::new();
        event.record(&mut visitor);

        self.log(&event.metadata().into(), visitor);
    }
}

impl Subscriber for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= self.level
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        assert!(id != 0);

        let id = unsafe { Id::from_non_zero_u64(NonZeroU64::new_unchecked(id)) };

        self.spans
            .write()
            .insert(id.clone(), span.metadata().into());
        id
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut visitor = Visitor::new();
        values.record(&mut visitor);

        // self.log(Level::INFO, visitor);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = Visitor::new();
        event.record(&mut visitor);

        self.log(&event.metadata().into(), visitor);
    }

    fn enter(&self, span: &Id) {
        let spans = self.spans.read();
        let Some(details) = spans.get(span) else {
            return;
        };

        self.log(details, format!("--> {}", span.into_u64()));
    }

    fn exit(&self, span: &Id) {
        let spans = self.spans.read();
        let Some(details) = spans.get(span) else {
            return;
        };

        self.log(details, format!("<-- {}", span.into_u64()));
    }

    fn clone_span(&self, id: &Id) -> Id {
        if let Some(details) = self.spans.write().get(id) {
            details.inc();
        }

        id.clone()
    }

    fn try_close(&self, id: Id) -> bool {
        let mut spans = self.spans.write();

        if let Some(details) = spans.get(&id) {
            if details.dec() {
                spans.remove(&id);
                true
            } else {
                false
            }
        } else {
            false
        }
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
        let _ = match field.name() {
            "message" => write!(self.buf, "{:?}", value),
            _ => write!(self.buf, "{} = {:?}", field, value),
        };
    }
}

impl Display for Visitor {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.buf, f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct Color([u8; 2]);

impl Color {
    // Bold
    pub const RED: Self = Self([1, 31]);
    pub const YELLOW: Self = Self([1, 33]);
    pub const GREEN: Self = Self([1, 32]);
    pub const LIGHT_GRAY_BOLD: Self = Self([1, 37]);

    pub const LIGHT_GRAY: Self = Self([2, 37]);
}

struct ColorText<T>
where
    T: Display,
{
    text: T,
    color: Color,
}

impl<T> ColorText<T>
where
    T: Display,
{
    fn new(text: T, color: Color) -> Self {
        Self { text, color }
    }
}

impl<T> Display for ColorText<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\x1b[{};{}m{}\x1b[0m",
            self.color.0[0], self.color.0[1], self.text
        )
    }
}

#[derive(Debug)]
struct SpanDetails {
    name: String,
    level: Level,
    count: AtomicU64,
}

impl SpanDetails {
    fn inc(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the RC and returns `true` if the last span was dropped.
    fn dec(&self) -> bool {
        if self.count.fetch_sub(1, Ordering::Release) != 1 {
            return false;
        }

        self.count.load(Ordering::Acquire);
        true
    }
}

impl<'a> From<&'a Metadata<'a>> for SpanDetails {
    fn from(value: &'a Metadata<'a>) -> Self {
        let name = value.module_path().unwrap_or("???").to_owned();

        Self {
            name,
            level: *value.level(),
            count: AtomicU64::new(1),
        }
    }
}
