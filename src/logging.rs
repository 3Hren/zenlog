use std::io::{Write};

use libc;
use chrono;
use log;
use log::{LogRecord, LogLevel, LogMetadata, SetLoggerError};
use termion::{TermWrite};
use termion::color::{self, Palette};

struct Logger {
    level: LogLevel,
}

impl Logger {
    fn new(level: LogLevel) -> Logger {
        Logger {
            level: level,
        }
    }
}

fn severity(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "T",
        LogLevel::Debug => "D",
        LogLevel::Info  => "I",
        LogLevel::Warn  => "W",
        LogLevel::Error => "E",
    }
}

fn color(level: LogLevel) -> Palette {
    match level {
        LogLevel::Trace |
        LogLevel::Debug => Palette::Rgb(2, 2, 2),
        LogLevel::Info  => Palette::Rgb(0, 1, 3),
        LogLevel::Warn  => Palette::Yellow,
        LogLevel::Error => Palette::Red,
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.level && metadata.target().starts_with("zenlog")
    }

    fn log(&self, rec: &LogRecord) {
        if self.enabled(rec.metadata()) {
            let stdout = ::std::io::stdout();
            let mut wr = stdout.lock();

            let now = chrono::UTC::now();

            wr.color(Palette::Rgb(2, 2, 2)).unwrap();
            write!(wr, "{} ", now).unwrap();
            wr.color(color(rec.level())).unwrap();
            write!(wr, "{} {}/", severity(rec.level()), unsafe { libc::getpid() }).unwrap();
            write!(wr, "{}:{:<4}", rec.location().module_path(), rec.location().line()).unwrap();
            wr.color(color::White).unwrap();
            write!(wr, " - {}\r\n", rec.args()).unwrap();
            wr.reset().unwrap();
            wr.flush().unwrap();
        }
    }
}

pub fn from_usize(v: usize) -> LogLevel {
    match v {
        0 => LogLevel::Trace,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        _ => LogLevel::Error,
    }
}

pub fn reset(level: LogLevel) -> Result<(), SetLoggerError> {
    log::set_logger(|max| {
        max.set(level.to_log_level_filter());
        Box::new(Logger::new(level))
    })
}
