use std::io::{stdout, Write};
use std::sync::Arc;

use chrono::{DateTime, UTC};
use chrono::naive::datetime::NaiveDateTime;

use termion::color::{self, AnsiValue};

use {Config, Record};
use output::{Output, OutputFactory};

/// Eye-candy output that is used mainly both for demonstrating `Zen` features and for developing
/// applications.
///
/// This output should be only used for developing reasons.
///
/// # Note
///
/// This output is activated automatically when `Zen` is executed without arguments.
pub struct Dev;

impl Dev {
    fn new() -> Dev {
        Dev
    }
}

const NANOSECONDS_IN_SECOND: i64 = 1000000000;

fn color_from_severity(sev: char) -> color::Fg<AnsiValue> {
    let rgb = match sev {
        'E' => AnsiValue::rgb(2, 0, 0),
        'W' => AnsiValue::rgb(4, 3, 0),
        'I' => AnsiValue::rgb(0, 1, 3),
        _ => AnsiValue::rgb(2, 2, 2),
    };

    color::Fg(rgb)
}

impl Output for Dev {
    fn handle(&mut self, record: &Arc<Record>) {
        let wr = stdout();
        let mut wr = wr.lock();

        write!(wr, "{}", color::Fg(AnsiValue::rgb(2, 2, 2))).unwrap();
        if let Some(val) = record.find("timestamp") {
            if let Some(val) = val.as_i64() {
                let msecs = val % NANOSECONDS_IN_SECOND;
                let timestamp = val / NANOSECONDS_IN_SECOND;

                match NaiveDateTime::from_timestamp_opt(timestamp, msecs as u32) {
                    Some(datetime) => {
                        let ts: DateTime<UTC> = DateTime::from_utc(datetime, UTC);
                        write!(wr, "{}", ts.format("%Y-%m-%d %H:%M:%S%.6f %Z")).unwrap();
                    }
                    None => {
                        warn!("failed to convert {} value into datetime", val);
                        return;
                    }
                }
            } else if let Some(val) = val.as_string() {
                write!(wr, "{}", val).unwrap();
            } else {
                unimplemented!();
            }
        }

        if let Some(sev) = record.find("levelname").and_then(|v| v.as_string()) {
            if let Some(ch) = sev.chars().next() {
                let color = color_from_severity(ch);

                write!(wr, "{} {}", color, ch).unwrap();
            }
        }

        let pid = record.find("pid").and_then(|v| v.as_u64());
        let tid = record.find("tid").and_then(|v| v.as_u64());

        match (pid, tid) {
            (Some(pid), Some(tid)) => {
                write!(wr, " {:6.6}/{:#014.14x}", pid, tid).unwrap();
            }
            (Some(pid), None) => {
                write!(wr, " {:6.6}", pid).unwrap();
            }
            (None, Some(..)) | (None, None) => {}
        }

        let module = record.find("module").and_then(|v| v.as_string());
        let line = record.find("lineno").and_then(|v| v.as_u64());

        if let (Some(module), Some(line)) = (module, line) {
            write!(wr, " {:>14.14}:{:3}", module, line).unwrap();
        }

        let trcid = record.find("trace_id").and_then(|v| v.as_u64());
        let spnid = record.find("span_id").and_then(|v| v.as_u64());
        let parid = record.find("parent_id").and_then(|v| v.as_u64());

        if let (Some(trcid), Some(spnid), Some(parid)) = (trcid, spnid, parid) {
            write!(wr, " [{:6.6}:{:6.6}:{:6.6}]",
                format!("{:#06.6x}", trcid),
                format!("{:#06.6x}", spnid),
                format!("{:#06.6x}", parid)
            ).unwrap();
        } else if let Some(trcid) = trcid {
            write!(wr, " [{:6.6}:{:6}:{:6}]", format!("{:#0x}", trcid), ' ', ' ').unwrap();
        } else {
            write!(wr, " [{:6}:{:6}:{:6}]", ' ', ' ', ' ').unwrap();
        }

        if let Some(val) = record.find("message") {
            if let Some(val) = val.as_string() {
                write!(wr, " - {}{}", color::Fg(color::White), val).unwrap();
            }
        }

        write!(wr, "\r\n").unwrap();
    }
}

impl OutputFactory for Dev {
    type Error = &'static str;

    fn ty() -> &'static str {
        "dev"
    }

    #[allow(unused_variables)]
    fn from(cfg: &Config) -> Result<Box<Output>, Self::Error> {
        Ok(Box::new(Dev::new()))
    }
}
