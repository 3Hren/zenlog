use std::fs::File;
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
    outputs: Vec<Value>,
}

impl PipeConfig {
    pub fn sources(&self) -> &Vec<Value> {
        &self.sources
    }

    pub fn outputs(&self) -> &Vec<Value> {
        &self.outputs
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Logging severity.
    severity: usize,
    /// Generic pipelines config.
    pipelines: Vec<PipeConfig>,
}

impl Config {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let data = {
            let mut data = String::new();
            let mut file = File::open(path)?;
            file.read_to_string(&mut data)?;

            data
        };

        let result = serde_yaml::from_str(&data)?;

        // if result.pipeline.is_empty() {
        //     return Err(Error::EmptyPipe);
        // }

        Ok(result)
    }

    pub fn severity(&self) -> usize {
        self.severity
    }

    pub fn pipelines(&self) -> &Vec<PipeConfig> {
        &self.pipelines
    }
}

#[cfg(test)]
mod test {}
