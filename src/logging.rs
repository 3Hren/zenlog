//! In the future it should contain optional TRACE level logging.

use std::io::{Write};

use chrono;
use log;
use log::{LogRecord, LogLevel, LogMetadata, SetLoggerError};
use term;
use term::{Terminal, TerminfoTerminal};

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

fn color(level: LogLevel) -> term::color::Color {
    match level {
        LogLevel::Trace |
        LogLevel::Debug => term::color::WHITE,
        LogLevel::Info  => term::color::BLUE,
        LogLevel::Warn  => term::color::YELLOW,
        LogLevel::Error => term::color::RED,
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let now = chrono::UTC::now();

            let mut term = TerminfoTerminal::new(Vec::new()).unwrap();

            term.fg(color(record.level())).unwrap();
            write!(term, "{}", severity(record.level())).unwrap();
            term.reset().unwrap();

            write!(term, ", {} {:<32}: ", now, record.target()).unwrap();

            term.fg(color(record.level())).unwrap();
            write!(term, "{}\n", record.args()).unwrap();
            term.reset().unwrap();
            term.get_mut().flush().unwrap();

            let stdout = ::std::io::stdout();
            let mut wr = stdout.lock();
            wr.write(term.get_ref()).unwrap();
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
