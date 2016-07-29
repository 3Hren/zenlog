use std::error::Error;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono;
use libc;
use log::{self, LogRecord, LogLevel, LogMetadata};
use termion::color::{self, AnsiValue};

fn level_as_usize(level: LogLevel) -> usize {
    match level {
        LogLevel::Error => 0,
        LogLevel::Warn => 1,
        LogLevel::Info => 2,
        LogLevel::Debug => 3,
        LogLevel::Trace => 4,
    }
}

fn level_as_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARN",
        LogLevel::Info => "INFO",
        LogLevel::Debug => "DEBUG",
        LogLevel::Trace => "TRACE",
    }
}

struct Logger {
    level: Arc<AtomicUsize>,
}

impl Logger {
    fn new(level: Arc<AtomicUsize>) -> Logger {
        Logger {
            level: level,
        }
    }
}

fn color(level: LogLevel) -> color::Fg<AnsiValue> {
    let val = match level {
        LogLevel::Trace |
        LogLevel::Debug => AnsiValue::rgb(2, 2, 2),
        LogLevel::Info  => AnsiValue::rgb(0, 1, 3),
        LogLevel::Warn  => AnsiValue::rgb(4, 3, 0),
        LogLevel::Error => AnsiValue::rgb(2, 0, 0),
    };

    color::Fg(val)
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        level_as_usize(metadata.level()) <= self.level.load(Ordering::Relaxed) &&
            metadata.target().starts_with("zenlog")
    }

    fn log(&self, rec: &LogRecord) {
        if self.enabled(rec.metadata()) {
            let stdout = ::std::io::stdout();
            let mut wr = stdout.lock();

            let now = chrono::UTC::now();
            let pid = unsafe { libc::getpid() };

            writeln!(wr, "{} {} {}{} {}/{}:{:<4}{} - {}{}{}",
                color::Fg(AnsiValue::rgb(2, 2, 2)),
                now,
                color(rec.level()),
                level_as_str(rec.level()).chars().next().unwrap(),
                pid,
                rec.location().module_path(),
                rec.location().line(),
                color::Fg(AnsiValue::rgb(2, 2, 2)),
                color::Fg(color::White),
                rec.args(),
                color::Fg(color::Reset)
            ).unwrap();
            wr.flush().unwrap();
        }
    }
}

pub trait AsUsize {
    fn as_usize(&self) -> usize;
}

impl AsUsize for LogLevel {
    fn as_usize(&self) -> usize {
        level_as_usize(*self)
    }
}

pub trait AsLogLevel {
    fn as_level(&self) -> Option<LogLevel>;
}

impl AsLogLevel for usize {
    fn as_level(&self) -> Option<LogLevel> {
        let level = match *self {
            0 => LogLevel::Error,
            1 => LogLevel::Warn,
            2 => LogLevel::Info,
            3 => LogLevel::Debug,
            _ => LogLevel::Trace,
        };

        Some(level)
    }
}

impl AsLogLevel for str {
    fn as_level(&self) -> Option<LogLevel> {
        match self {
            "ERROR" => Some(LogLevel::Error),
            "WARN"  => Some(LogLevel::Warn),
            "INFO"  => Some(LogLevel::Info),
            "DEBUG" => Some(LogLevel::Debug),
            "TRACE" => Some(LogLevel::Trace),
            _ => None,
        }
    }
}

/// Initializes the logging system.
pub fn init<T: ?Sized + AsLogLevel>(level: &T) -> Result<Arc<AtomicUsize>, Box<Error>> {
    let level = try!(level.as_level().ok_or("invalid severity level"));
    let lvl = Arc::new(AtomicUsize::new(level_as_usize(level)));

    let clone = lvl.clone();
    log::set_logger(|max| {
        max.set(level.to_log_level_filter());
        Box::new(Logger::new(clone))
    })
    .map(|_| lvl)
    .map_err(|e| e.into())
}
