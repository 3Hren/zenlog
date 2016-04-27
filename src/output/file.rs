use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use serde_json;

use super::Output;
use super::super::{Record};

#[derive(Debug, Copy, Clone, PartialEq)]
enum ParserError {
    EOFWhileParsingPlaceholder,
}

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Literal(String),
    Placeholder(Vec<String>),
    Error(ParserError),
}

#[derive(Debug, PartialEq)]
enum State {
    Undefined,           // At start or after parsing value in streaming mode.
    ParsePlaceholder,    // Just after literal.
    Broken(ParserError), // Just after any error, meaning the parser will always fail from now.
}

struct Parser<T> {
    rd: T,
    state: State,
}

impl<T: Iterator<Item = char>> Parser<T> {
    fn new(rd: T) -> Parser<T> {
        Parser {
            rd: rd,
            state: State::Undefined
        }
    }

    fn parse(&mut self) -> Option<Event> {
        match self.rd.next() {
            Some('{') => { self.parse_placeholder() }
            Some(ch)  => { self.parse_literal(ch) }
            None      => { None }
        }
    }

    fn parse_literal(&mut self, ch: char) -> Option<Event> {
        let mut result = String::new();
        result.push(ch);

        loop {
            match self.rd.next() {
                Some('{') => {
                    self.state = State::ParsePlaceholder;
                    break
                }
                Some(ch) => { result.push(ch) }
                None => { break }
            }
        }

        Some(Event::Literal(result))
    }

    fn parse_placeholder(&mut self) -> Option<Event> {
        let mut result = String::new();

        loop {
            match self.rd.next() {
                Some('}') => {
                    self.state = State::Undefined;
                    return Some(Event::Placeholder(result.split('.').map(String::from).collect()));
                }
                Some(c) => { result.push(c) }
                None    => {
                    self.state = State::Broken(ParserError::EOFWhileParsingPlaceholder);
                    return Some(Event::Error(ParserError::EOFWhileParsingPlaceholder));
                }
            }
        }
    }
}

impl<T: Iterator<Item = char>> Iterator for Parser<T> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        match self.state {
            State::Undefined        => self.parse(),
            State::ParsePlaceholder => self.parse_placeholder(),
            State::Broken(err)      => Some(Event::Error(err)),
        }
    }
}

#[derive(Debug)]
enum TokenError {
    KeyNotFound(String),
    Serialize(serde_json::error::Error),
}

fn consume(token: &Token, record: &Record) -> Result<String, TokenError> {
    use serde_json::Value;

    match *token {
        Token::Literal(ref value) => { Ok(value.clone()) }
        Token::Placeholder(ref placeholders) => {
            let mut current = record;
            for key in placeholders {
                match current.find(key) {
                    Some(v) => { current = v; }
                    None    => { return Err(TokenError::KeyNotFound(key.clone())); }
                }
            }

            serde_json::to_string(current).map_err(|e| TokenError::Serialize(e))
        }
    }
}

enum Token {
    Literal(String),
    Placeholder(Vec<String>),
}

impl Token {
    fn from(event: Event) -> Result<Token, ParserError> {
        match event {
            Event::Literal(v) => Ok(Token::Literal(v)),
            Event::Placeholder(v) => Ok(Token::Placeholder(v)),
            Event::Error(err) => Err(err),
        }
    }
}

/// File output will write log events to files on disk.
///
/// Path can contain placeholders. For example: test.log, {source}.log, {source.host}.log
/// It creates directories and files (with append mode) automatically.
/// Log format: {timestamp} {message} by default. Can contain any attributes.
/// If attribute not found - drop event and warn.
struct FilesWriter {
    path: Vec<Token>,
    pattern: Vec<Token>,

    files: HashMap<PathBuf, File>,
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Path(err: ParserError) {}
        Pattern(err: ParserError) {}
    }
}

impl FilesWriter {
    /// # Fails
    ///
    /// Function may fail if either of the given path or pattern is invalid.
    fn new(path: &str, pattern: &str) -> Result<FilesWriter, Error> {
        let path = Parser::new(path.chars())
            .map(Token::from)
            .collect::<Result<Vec<Token>, ParserError>>()
            .map_err(Error::Path)?;

        let pattern = Parser::new(pattern.chars())
            .map(Token::from)
            .collect::<Result<Vec<Token>, ParserError>>()
            .map_err(Error::Pattern)?;

        let result = FilesWriter {
            path: path,
            pattern: pattern,
            files: HashMap::new(),
        };

        Ok(result)
    }

    fn path(&self, record: &Record) -> Result<String, TokenError> {
        // TODO: Use collect with result collapsing.
        let mut path = String::new();
        for token in &self.path {
            path.push_str(&consume(&token, record)?);
        }

        Ok(path)
    }

    // TODO: Return Result.
    fn write(&mut self, record: &Record) -> Result<(), ()> {
        let path = match self.path(record) {
            Ok(path) => path,
            Err(err) => {
                // TODO: return Err(Error::ParserError);
                warn!("dropped {:?} while parsing path format - {:?}", record, err);
                return Err(());
            }
        };

        let path = Path::new(&path);
        let file = match self.files.entry(path.to_path_buf()) {
            Entry::Vacant(entry) => {
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
        for token in &self.pattern {
            let token = match consume(token, record) {
                Ok(token) => token,
                Err(err) => {
                    error!("dropping {:?} while parsing message format - {:?}", record, err);
                    return Err(());
                }
            };
            message.push_str(&token);
        }
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

pub struct FileOutput {
    tx: mpsc::Sender<Arc<Record>>,
}

impl FileOutput {
    pub fn new<P: AsRef<Path>>(path: P) -> FileOutput {
        let (tx, rx) = mpsc::channel::<Arc<Record>>();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        thread::spawn(move || {
            for record in rx {
                let mut message = String::new();
                message.push_str(record.find("message").unwrap().as_string().unwrap());
                message.push('\n');

                match file.write(message.as_bytes()) {
                    Ok(num) => debug!("{} bytes written", num),
                    Err(err) => error!("writing error - {}", err),
                }
            }
        });

        FileOutput {
            tx: tx,
        }
    }
}

impl Output for FileOutput {
    fn handle(&mut self, record: &Arc<Record>) {
        self.tx.send(record.clone()).unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::{Event, Parser, ParserError};

    #[test]
    fn parse_empty_path() {
        let mut parser = Parser::new("".chars());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn parse_literal() {
        let mut parser = Parser::new("file.log".chars());
        assert_eq!(Some(Event::Literal("file.log".to_string())), parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn parse_placeholder() {
        let mut parser = Parser::new("{id}".chars());
        assert_eq!(Some(Event::Placeholder(vec!["id".to_string()])), parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn parse_placeholder_nested() {
        let mut parser = Parser::new("{id.source}".chars());
        assert_eq!(Some(Event::Placeholder(vec!["id".to_string(), "source".to_string()])), parser.next());
        assert_eq!(None, parser.next());
    }

     #[test]
    fn parse_literal_placeholder() {
        let mut parser = Parser::new("/directory/file.{log}".chars());
        assert_eq!(Some(Event::Literal("/directory/file.".to_string())), parser.next());
        assert_eq!(Some(Event::Placeholder(vec!["log".to_string()])), parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn parse_placeholder_literal() {
        let mut parser = Parser::new("{directory}/file.log".chars());
        assert_eq!(Some(Event::Placeholder(vec!["directory".to_string()])), parser.next());
        assert_eq!(Some(Event::Literal("/file.log".to_string())), parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn parse_literal_placeholder_literal() {
        let mut parser = Parser::new("/directory/{path}.log".chars());
        assert_eq!(Some(Event::Literal("/directory/".to_string())), parser.next());
        assert_eq!(Some(Event::Placeholder(vec!["path".to_string()])), parser.next());
        assert_eq!(Some(Event::Literal(".log".to_string())), parser.next());
        assert_eq!(None, parser.next());
    }

    #[test]
    fn break_parser_on_eof_while_parsing_placeholder() {
        let mut parser = Parser::new("/directory/{path".chars());
        assert_eq!(Some(Event::Literal("/directory/".to_string())), parser.next());
        assert_eq!(Some(Event::Error(ParserError::EOFWhileParsingPlaceholder)), parser.next());
        assert_eq!(Some(Event::Error(ParserError::EOFWhileParsingPlaceholder)), parser.next());
    }
}
