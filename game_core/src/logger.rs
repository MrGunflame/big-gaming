use std::fmt::Write;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::IsTerminal;

use chrono::Local;
use game_tracing::{ProfilerConfig, ProfilingLayer};
use tracing::field::{Field, Visit};
use tracing::metadata::LevelFilter;
use tracing::subscriber::set_global_default;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub fn init() {
    let layer = tracing_subscriber::registry();
    #[cfg(feature = "tracy")]
    let layer = layer.with(ProfilingLayer::new(ProfilerConfig::default()));
    let layer = layer.with(Logger::new());

    set_global_default(layer).unwrap();
}

#[derive(Debug)]
pub struct Logger {
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

        Self { is_tty, level }
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

        // Most messages are between 100 and 200 bytes, so this
        // value means we have to never reallocate for short messages
        // and only once for long messages.
        let mut buf = String::with_capacity(128);

        let now = Local::now().format("%Y-%m-%d %H:%M:%S:%f");

        let (level, color) = match *event.metadata().level() {
            Level::ERROR => ("ERROR", Color::RED),
            Level::WARN => ("WARN", Color::YELLOW),
            Level::INFO => ("INFO", Color::GREEN),
            Level::DEBUG => ("DEBUG", Color::LIGHT_GRAY_BOLD),
            Level::TRACE => ("TRACE", Color::LIGHT_GRAY_BOLD),
        };

        let name = event.metadata().module_path().unwrap_or("???");

        if self.is_tty {
            write!(
                buf,
                "{} {} {} ",
                ColorText::new(format_args!("[{}]", now), Color::LIGHT_GRAY),
                ColorText::new(level, color),
                ColorText::new(name, Color::LIGHT_GRAY),
            )
            .ok();
        } else {
            write!(buf, "[{}] {} {} ", now, level, name).ok();
        };

        let mut visitor = Visitor::new(&mut buf);
        event.record(&mut visitor);

        println!("{}", buf);
    }
}

struct Visitor<W> {
    writer: W,
}

impl<W> Visitor<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> Visit for Visitor<W>
where
    W: Write,
{
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        match field.name() {
            "message" => write!(self.writer, "{:?}", value).ok(),
            _ => write!(self.writer, "{} = {:?}", field, value).ok(),
        };
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
