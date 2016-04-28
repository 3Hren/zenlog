use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

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

/// File output will write log events to files on disk.
///
/// Path can contain placeholders. For example: test.log, {source}.log, {source.host}.log
/// It creates directories and files (with append mode) automatically.
/// Log format: {timestamp} {message} by default. Can contain any attributes.
/// If attribute not found - drop event and warn.
struct FilesWriter<'a> {
    path: Generator<'a>,
    pattern: Generator<'a>,
    files: HashMap<PathBuf, File>,
}

impl<'a> FilesWriter<'a> {
    /// # Fails
    ///
    /// Function may fail if either of the given path or pattern is invalid.
    fn new(path: &'a str, pattern: &'a str) -> Result<FilesWriter<'a>, Error> {
        let result = FilesWriter {
            path: Generator::new(path).map_err(Error::Path)?,
            pattern: Generator::new(pattern).map_err(Error::Pattern)?,
            files: HashMap::new(),
        };

        Ok(result)
    }

    // TODO: Return Result.
    fn write(&mut self, record: &Record) -> Result<(), ()> {
        let path = match self.path.consume(record) {
            Ok(path) => path,
            Err(err) => {
                // TODO: return Err(Error::ParserError);
                error!("dropped {:?} while parsing path format - {:?}", record, err);
                return Err(());
            }
        };

        let path = Path::new(&path);
        let file = match self.files.entry(path.to_path_buf()) {
            Entry::Vacant(entry) => {
                // TODO: Create all subdirectories.
                let file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(path)
                    .unwrap();
                info!("opened '{}' for writing in append mode", path.display());

                entry.insert(file)
            }
            Entry::Occupied(entry) => entry.into_mut(),
        };

        let mut message = String::new();
        match self.pattern.consume(&record) {
            Ok(result) => message.push_str(&result),
            Err(err) => {
                error!("dropped {:?} while parsing message format - {:?}", record, err);
                return Err(());
            }
        };
        message.push('\n');

        match file.write_all(message.as_bytes()) {
            Ok(()) => debug!("{} bytes written", message.len()),
            Err(err) => error!("writing error - {:?}", err)
        }

        Ok(())
    }

    fn reopen(&mut self) {
        unimplemented!();
    }
}

pub struct FilePattern {
    tx: mpsc::Sender<Arc<Record>>,
}

impl FilePattern {
    pub fn new(path: String, pattern: String) -> Result<FilePattern, Error> {
        let (tx, rx) = mpsc::channel::<Arc<Record>>();

        thread::spawn(move || {
            let mut output = FilesWriter::new(&path, &pattern).unwrap();

            for record in rx {
                if let Err(err) = output.write(&record) {
                    error!("failed to write {:?}: {:?}", record, err);
                }
            }
        });

        let result = FilePattern {
            tx: tx,
        };

        Ok(result)
    }
}

impl Output for FilePattern {
    fn ty() -> &'static str where Self: Sized {
        "files"
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

impl OutputFrom for FilePattern {
    type Error = Error;
    type Config = Config;

    fn from(config: Config) -> Result<FilePattern, Error> {
        let Config { path, pattern } = config;

        FilePattern::new(path, pattern)
    }
}
