use std::fs::{File};
use std::io::Read;
use std::path::Path;

use serde_yaml;
use serde_json::Value;

quick_error! {
    /// Configuration reading, parsing and sanity errors.
    #[derive(Debug)]
    pub enum Error {
        Io(err: ::std::io::Error) {
            from()
        }
        Yaml(err: serde_yaml::error::Error) {
            from()
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipeConfig {
    sources: Vec<Value>,
    // TODO: Filter.
    // TODO: Output.
}

impl PipeConfig {
    pub fn sources(&self) -> &Vec<Value> {
        &self.sources
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Logging severity.
    severity: usize,
    /// Generic pipelines config.
    // TODO: Consider better naming for a variable.
    pipeline: Vec<PipeConfig>,
}

impl Config {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let data = {
            let mut data = String::new();
            let mut file = try!(File::open(path));
            try!(file.read_to_string(&mut data));

            data
        };

        let result = try!(serde_yaml::from_str(&data));

        // if result.pipeline.is_empty() {
        //     return Err(Error::EmptyPipe);
        // }

        Ok(result)
    }

    pub fn severity(&self) -> usize {
        self.severity
    }

    pub fn pipeline(&self) -> &Vec<PipeConfig> {
        &self.pipeline
    }
}

#[cfg(test)]
mod test {}
