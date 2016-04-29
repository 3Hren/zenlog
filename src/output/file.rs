use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::thread;

use rtfmt;
use rtfmt::{Generator, ParseError};

use super::{Output, OutputFrom};
use super::super::{Record};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Path(err: ParseError) {}
        Pattern(err: ParseError) {}
    }
}

#[derive(Debug)]
pub enum WriteError<'a> {
    Format(rtfmt::Error<'a>),
    Io(::std::io::Error),
}

impl<'a> From<rtfmt::Error<'a>> for WriteError<'a> {
    fn from(err: rtfmt::Error<'a>) -> WriteError {
        WriteError::Format(err)
    }
}

impl<'a> From<::std::io::Error> for WriteError<'a> {
    fn from(err: ::std::io::Error) -> WriteError<'a> {
        WriteError::Io(err)
    }
}

struct Writer<'a> {
    path: Generator<'a>,
    pattern: Generator<'a>,
    files: HashMap<PathBuf, File>,
}

impl<'a> Writer<'a> {
    /// # Fails
    ///
    /// Function may fail if either of the given path or pattern is invalid.
    fn new(path: &'a str, pattern: &'a str) -> Result<Writer<'a>, Error> {
        let result = Writer {
            path: Generator::new(path).map_err(Error::Path)?,
            pattern: Generator::new(pattern).map_err(Error::Pattern)?,
            files: HashMap::new(),
        };

        Ok(result)
    }

    fn write(&mut self, record: &Record) -> Result<(), WriteError> {
        let path = self.path.consume(record)?;
        let path = Path::new(&path);
        let file = match self.files.entry(path.to_path_buf()) {
            Entry::Vacant(entry) => {
                // Ensure that path components except the last one - is dir (or doesn't exist).
                let mut components = path.components();
                components.next_back();

                let parent_path = components.as_path();
                if parent_path.exists() {
                    if !parent_path.is_dir() {
                        unimplemented!();
                    }
                } else {
                    DirBuilder::new().recursive(true).create(parent_path)?;
                }

                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)?;
                info!("opened '{}' for writing in append mode", path.display());

                entry.insert(file)
            }
            Entry::Occupied(entry) => entry.into_mut(),
        };

        let mut buf = Vec::new();
        self.pattern.feed(&mut buf, &record)?;
        buf.push(10);

        match file.write_all(&buf) {
            Ok(()) => debug!("{} bytes written", buf.len()),
            Err(err) => error!("writing error - {:?}", err)
        }

        Ok(())
    }

    fn reopen(&mut self) {
        unimplemented!();
    }
}

/// Represents file output that writes events to files on disk.
///
/// This kind of output accepts incoming record, generates an individual path from the configuration
/// allowing to use fields from the record as parts of the filename and/or path and formats it
/// using specified format and writes the result into the file. If a required field was not found
/// the event will be dropped with error log.
///
/// Note that all subdirectories and files (with append mode) will be created automatically on
/// demand.
pub struct FileOutput {
    tx: mpsc::Sender<Arc<Record>>,
}

impl FileOutput {
    pub fn new(path: String, pattern: String) -> Result<FileOutput, Error> {
        let (tx, rx) = mpsc::channel::<Arc<Record>>();

        thread::spawn(move || {
            let mut output = Writer::new(&path, &pattern).unwrap();

            for record in rx {
                if let Err(err) = output.write(&record) {
                    error!("failed to write {:?}: {:?}", record, err);
                }
            }
        });

        let res = FileOutput {
            tx: tx,
        };

        Ok(res)
    }
}

impl Output for FileOutput {
    fn ty() -> &'static str where Self: Sized {
        "file"
    }

    fn handle(&mut self, record: &Arc<Record>) {
        self.tx.send(record.clone()).unwrap();
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    path: String,
    pattern: String,
}

impl OutputFrom for FileOutput {
    type Error = Error;
    type Config = Config;

    fn from(config: Config) -> Result<FileOutput, Error> {
        let Config { path, pattern } = config;

        FileOutput::new(path, pattern)
    }
}
