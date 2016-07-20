use std::error::Error;
use std::fs::File;
use std::path::Path;

use serde_json;

pub type Value = serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct PipeConfig {
    sources: Vec<Value>,
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
pub struct RuntimeConfig {
    /// Logging severity.
    severity: usize,
    /// Generic pipelines config.
    pipelines: Vec<PipeConfig>,
}

impl RuntimeConfig {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<RuntimeConfig, Box<Error>> {
        let cfg = try!(serde_json::from_reader(&try!(File::open(path))));

        Ok(cfg)
    }

    pub fn severity(&self) -> usize {
        self.severity
    }

    pub fn pipelines(&self) -> &Vec<PipeConfig> {
        &self.pipelines
    }
}
