use std::backtrace::Backtrace;
use std::fmt::Write;
use std::fmt::{self, Debug, Display, Formatter};
use std::fs::File;
use std::io::{self, IsTerminal, Write as _};
use std::panic::PanicHookInfo;
use std::sync::OnceLock;

use chrono::{DateTime, Local};
use parking_lot::Mutex;
use tracing::field::{Field, Visit};
use tracing::metadata::LevelFilter;
use tracing::subscriber::set_global_default;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init() {
    let layer = tracing_subscriber::registry();
    #[cfg(feature = "tracy")]
    let layer = layer.with(game_tracing::TracyLayer::new());

    let logger = LOGGER.get_or_init(Logger::new);
    let layer = layer.with(logger);

    std::panic::set_hook(Box::new(panic_hook));

    set_global_default(layer).unwrap();
}

#[derive(Debug)]
pub struct Logger {
    is_tty: bool,
    level: LevelFilter,
    file_logger: Mutex<Option<FileLogger>>,
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

        let is_tty = io::stdout().is_terminal();

        let log_path = match std::env::var("GAME_LOG_PATH") {
            Ok(val) if val.to_lowercase() == "none" => None,
            Ok(val) => Some(val),
            Err(_) => Some("game.log".to_owned()),
        };

        let file_logger = match log_path {
            Some(path) => match File::create(path) {
                Ok(file) => Some(FileLogger::new(file)),
                Err(err) => {
                    println!("Failed to create file for logging: {}", err);
                    None
                }
            },
            None => None,
        };

        Self {
            is_tty,
            level,
            file_logger: Mutex::new(file_logger),
        }
    }

    /// Returns a reference to the global `Logger` if it has been initialized.
    pub fn get() -> Option<&'static Self> {
        LOGGER.get()
    }

    /// Directly writes a `msg` to the output of the `Logger`.
    ///
    /// **Note: Do not use this as the default logging function. Use the [`tracing`] macros
    /// instead** which will pass through to this `Logger` implementation.
    ///
    /// The given `msg` is written as is (and no '\n' is appended).
    pub fn write(&self, msg: &str) {
        let mut stdout = io::stdout().lock();
        stdout.write_all(msg.as_bytes()).ok();
        drop(stdout);

        if let Some(file_logger) = &mut *self.file_logger.lock() {
            file_logger.write(msg).ok();
        }
    }

    /// Flushes the `Logger`, ensuring that previous [`write`] calls have been written to the
    /// output.
    ///
    /// [`write`]: Self::write
    pub fn flush(&self) {
        if let Some(file_logger) = &mut *self.file_logger.lock() {
            file_logger.flush().ok();
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for &'static Logger
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

        let now = Local::now();

        format_line(&mut buf, event, &now, self.is_tty);
        print!("{}", buf);

        buf.clear();
        format_line(&mut buf, event, &now, false);
        if let Some(file_logger) = &mut *self.file_logger.lock() {
            file_logger.write(&buf).ok();
        }
    }
}

fn format_line(mut buf: &mut String, event: &Event<'_>, now: &DateTime<Local>, is_tty: bool) {
    let now = now.format("%Y-%m-%d %H:%M:%S:%f");

    let (level, color) = match *event.metadata().level() {
        Level::ERROR => ("ERROR", Color::RED),
        Level::WARN => ("WARN", Color::YELLOW),
        Level::INFO => ("INFO", Color::GREEN),
        Level::DEBUG => ("DEBUG", Color::LIGHT_GRAY_BOLD),
        Level::TRACE => ("TRACE", Color::LIGHT_GRAY_BOLD),
    };

    let name = event.metadata().module_path().unwrap_or("???");

    if is_tty {
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

    buf.push('\n');
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

#[derive(Debug)]
struct FileLogger {
    file: File,
    buf: Vec<u8>,
}

impl FileLogger {
    fn new(file: File) -> Self {
        const BUFFER_SIZE: usize = 8192;

        Self {
            file,
            buf: Vec::with_capacity(BUFFER_SIZE),
        }
    }

    /// Queue a write of `msg` to the underlying file.
    ///
    /// The write may not happen immediately.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the write operation fails.
    ///
    /// [`Error`]: io::Error
    fn write(&mut self, msg: &str) -> io::Result<()> {
        let spare_cap = self.buf.capacity() - self.buf.len();
        if msg.len() > spare_cap {
            self.file.write_all(&self.buf)?;
            self.buf.clear();
        }

        // `msg` is too big for the buffer and needs
        // to be written directly.
        if msg.len() >= self.buf.capacity() {
            self.file.write_all(msg.as_bytes())?;
            return Ok(());
        }

        self.buf.extend_from_slice(msg.as_bytes());
        Ok(())
    }

    /// Flushes the `FileLogger`, ensuring that all previous writes have been written to the
    /// underlying file.
    ///
    /// # Errors
    ///
    /// This functions returns an [`Error`] if the final write or sync operation fails.
    ///
    /// [`Error`]: io::Error
    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            // write is signal-safe.
            // See https://www.man7.org/linux/man-pages/man7/signal-safety.7.html
            self.file.write_all(&self.buf)?;
        }

        // fsync is signal-safe.
        // See https://www.man7.org/linux/man-pages/man7/signal-safety.7.html
        self.file.sync_all()?;

        Ok(())
    }
}

fn panic_hook(info: &PanicHookInfo<'_>) {
    let backtrace = Backtrace::force_capture();

    let location = info.location().unwrap();
    let msg = if let Some(&s) = info.payload().downcast_ref::<&'static str>() {
        s
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s
    } else {
        "Box<dyn Any>"
    };

    let mut buf = String::with_capacity(4096);
    writeln!(buf, "thread panicked at {}:\n{}", location, msg).ok();
    writeln!(buf, "{}", backtrace).ok();

    let Some(logger) = Logger::get() else {
        eprintln!("panicked but logger is not installed");
        return;
    };

    logger.write(&buf);
    logger.flush();

    // Program should abort at this point as this
    // point at we don't unwind.
}
