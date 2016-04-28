use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use serde_json;
use rtfmt::{Generator, ParseError};
use rtfmt::Error as FormatError;

use super::Output;
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
    // pattern: Generator<'static>,

    // files: HashMap<PathBuf, File>,
}

impl<'a> FilesWriter<'a> {
    /// # Fails
    ///
    /// Function may fail if either of the given path or pattern is invalid.
    fn new(path: &'a str, pattern: &'a str) -> Result<FilesWriter<'a>, Error> {
        let result = FilesWriter {
            path: Generator::new(path).map_err(Error::Path)?,
        };

        // let path = Parser::new(path.chars())
        //     .map(Token::from)
        //     .collect::<Result<Vec<Token>, ParserError>>()
        //     .map_err(Error::Path)?;
        //
        // let pattern = Parser::new(pattern.chars())
        //     .map(Token::from)
        //     .collect::<Result<Vec<Token>, ParserError>>()
        //     .map_err(Error::Pattern)?;
        //
        // let result = FilesWriter {
        //     path: path,
        //     pattern: pattern,
        //     files: HashMap::new(),
        // };
        //
        // Ok(result)
        unimplemented!();
    }

    fn path(&self, record: &Record) -> Result<String, FormatError> {
        // // TODO: Use collect with result collapsing.
        // let mut path = String::new();
        // for token in &self.path {
        //     path.push_str(&consume(&token, record)?);
        // }
        //
        // Ok(path)
        unimplemented!();
    }

    // TODO: Return Result.
    fn write(&mut self, record: &Record) -> Result<(), ()> {
        unimplemented!();
        // let path = match self.path(record) {
        //     Ok(path) => path,
        //     Err(err) => {
        //         // TODO: return Err(Error::ParserError);
        //         warn!("dropped {:?} while parsing path format - {:?}", record, err);
        //         return Err(());
        //     }
        // };
        //
        // let path = Path::new(&path);
        // let file = match self.files.entry(path.to_path_buf()) {
        //     Entry::Vacant(entry) => {
        //         let file = OpenOptions::new()
        //             .write(true)
        //             .append(true)
        //             .create(true)
        //             .open(path)
        //             .unwrap();
        //         info!("opened '{}' for writing in append mode", path.display());
        //
        //         entry.insert(file)
        //     }
        //     Entry::Occupied(entry) => entry.into_mut(),
        // };
        //
        // let mut message = String::new();
        // for token in &self.pattern {
        //     let token = match consume(token, record) {
        //         Ok(token) => token,
        //         Err(err) => {
        //             error!("dropping {:?} while parsing message format - {:?}", record, err);
        //             return Err(());
        //         }
        //     };
        //     message.push_str(&token);
        // }
        // message.push('\n');
        //
        // match file.write_all(message.as_bytes()) {
        //     Ok(()) => debug!("{} bytes written", message.len()),
        //     Err(err) => error!("writing error - {:?}", err)
        // }
        //
        // Ok(())
    }

    fn reopen(&mut self) {
        unimplemented!();
    }
}

pub struct FilePattern {
    tx: mpsc::Sender<Arc<Record>>,
}

impl FilePattern {
    pub fn new(path: String, pattern: String) -> FilePattern {
        let (tx, rx) = mpsc::channel::<Arc<Record>>();

        thread::spawn(move || {
            let output = FilesWriter::new(&path, &pattern).unwrap();
        });

        FilePattern {
            tx: tx,
        }
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
