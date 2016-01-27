use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use super::super::Record;

#[derive(Debug, Copy, Clone, PartialEq)]
enum ParserError {
    EOFWhileParsingPlaceholder,
}

#[derive(Debug, Clone, PartialEq)]
enum ParserEvent {
    Literal(String),
    Placeholder(Vec<String>),
    Error(ParserError),
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Undefined,           // At start or after parsing value in streaming mode.
    ParsePlaceholder,    // Just after literal.
    Broken(ParserError), // Just after any error, meaning the parser will always fail from now.
}

struct FormatParser<T> {
    reader: T,
    state: ParserState,
}

impl<T: Iterator<Item = char>> FormatParser<T> {
    fn new(reader: T) -> FormatParser<T> {
        FormatParser {
            reader: reader,
            state: ParserState::Undefined
        }
    }

    fn parse(&mut self) -> Option<ParserEvent> {
        match self.reader.next() {
            Some('{') => { self.parse_placeholder() }
            Some(ch)  => { self.parse_literal(ch) }
            None      => { None }
        }
    }

    fn parse_literal(&mut self, ch: char) -> Option<ParserEvent> {
        let mut result = String::new();
        result.push(ch);

        loop {
            match self.reader.next() {
                Some('{') => {
                    self.state = ParserState::ParsePlaceholder;
                    break
                }
                Some(ch) => { result.push(ch) }
                None => { break }
            }
        }

        Some(ParserEvent::Literal(result))
    }

    fn parse_placeholder(&mut self) -> Option<ParserEvent> {
        let mut result = String::new();

        loop {
            match self.reader.next() {
                Some('}') => {
                    self.state = ParserState::Undefined;
                    let result = result.split('/').map(|s: &str| {
                        String::from(s)
                    }).collect();
                    return Some(ParserEvent::Placeholder(result));
                }
                Some(c) => { result.push(c) }
                None    => {
                    self.state = ParserState::Broken(ParserError::EOFWhileParsingPlaceholder);
                    return Some(ParserEvent::Error(ParserError::EOFWhileParsingPlaceholder));
                }
            }
        }
    }
}

impl<T: Iterator<Item = char>> Iterator for FormatParser<T> {
    type Item = ParserEvent;

    fn next(&mut self) -> Option<ParserEvent> {
        match self.state {
            ParserState::Undefined        => self.parse(),
            ParserState::ParsePlaceholder => self.parse_placeholder(),
            ParserState::Broken(err)      => Some(ParserEvent::Error(err)),
        }
    }
}

#[derive(Debug, PartialEq)]
enum TokenError<'r> {
    KeyNotFound(&'r str),
    TypeMismatch,
    SyntaxError(ParserError),
}

fn consume<'r>(event: &'r ParserEvent, record: &Record) -> Result<String, TokenError<'r>> {
    use serde_json::Value;

    match *event {
        ParserEvent::Literal(ref value) => { Ok(value.clone()) }
        ParserEvent::Placeholder(ref placeholders) => {
            let mut current = record;
            for key in placeholders.iter() {
                match current.find(key) {
                    Some(v) => { current = v; }
                    None    => { return Err(TokenError::KeyNotFound(&key[..])); }
                }
            }

            match *current {
                Value::Null => Ok("null".to_owned()),
                Value::Bool(v) => Ok(format!("{}", v)),
                Value::I64(v) => Ok(format!("{}", v)),
                Value::U64(v) => Ok(format!("{}", v)),
                Value::F64(v) => Ok(format!("{}", v)),
                Value::String(ref v) => Ok(v.clone()),
                Value::Array(..) => Err(TokenError::TypeMismatch),
                Value::Object(..) => Err(TokenError::TypeMismatch),
            }
        }
        ParserEvent::Error(err) => { Err(TokenError::SyntaxError(err)) }
    }
}

/// File output will write log events to files on disk.
///
/// Path can contain placeholders. For example: test.log, {source}.log, {source/host}.log
/// It creates directories and files (with append mode) automatically.
/// Log format: {timestamp} {message} by default. Can contain any attributes.
/// If attribute not found - drop event and warn.
pub struct FileOutput {
    path: Vec<ParserEvent>,
    message: Vec<ParserEvent>,
    files: HashMap<Path, File>,
}

impl FileOutput {
    pub fn new(path: &str, format: &str) -> FileOutput {
        FileOutput {
            path: FormatParser::new(path.chars()).collect(),
            message: FormatParser::new(format.chars()).collect(),
            files: HashMap::new(),
        }
    }
}

// impl Output for FileOutput {
//     fn feed(&mut self, payload: &Record) {
//         let mut path = String::new();
//         for token in self.path.iter() {
//             match consume(token, payload) {
//                 Ok(token) => path.push_str(token.as_slice()),
//                 Err(err) => {
//                     log!(Warn, "Output::File" -> "dropping {} while parsing path format - {}", payload, err);
//                     return;
//                 }
//             }
//         }
//
//         let path = Path::new(path);
//         let file = match self.files.entry(path.clone()) {
//             Vacant(entry) => {
//                 log!(Info, "Output::File" -> "opening file '{}' for writing in append mode", path.display());
//                 entry.set(File::open_mode(&path, Append, Write).unwrap())
//             }
//             Occupied(entry) => entry.into_mut(),
//         };
//
//         let mut message = String::new();
//         for token in self.message.iter() {
//             let token = match consume(token, payload) {
//                 Ok(token) => token,
//                 Err(err) => {
//                     log!(Warn, "Output::File" -> "dropping {} while parsing message format - {}", payload, err);
//                     return;
//                 }
//             };
//             message.push_str(token.as_slice());
//         }
//         message.push('\n');
//
//         match file.write(message.as_bytes()) {
//             Ok(())   => log!(Debug, "Output::File" -> "{} bytes written", message.len()),
//             Err(err) => log!(Warn, "Output::File" -> "writing error - {}", err)
//         }
//     }
// }


////////////////////////////////


// trait Output: Send {
//     fn feed(&mut self, payload: &Record);
// }
//
// impl Reflect for Output {}
//
// struct FileOutput {
//     file: File,
// }
//
// impl FileOutput {
//     pub fn new<P: AsRef<Path>>(path: P) -> FileOutput {
//         FileOutput {
//             file: OpenOptions::new()
//                 .write(true)
//                 .create(true)
//                 .append(true)
//                 .open(path)
//                 .unwrap(),
//         }
//     }
// }
//
// impl Output for FileOutput {
//     fn feed(&mut self, payload: &Record) {
//         let mut message = String::new();
//         message.push_str(payload.find("message").unwrap().as_string().unwrap());
//         message.push('\n');
//
//         match self.file.write(message.as_bytes()) {
//             Ok(num) => debug!(target: "FileOutput", "{} bytes written", num),
//             Err(err) => error!(target: "FileOutput", "writing error - {}", err),
//         }
//     }
// }
