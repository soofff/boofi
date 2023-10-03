use std::num::{ParseFloatError, ParseIntError};
use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct LoadAvg {
    avg1: f64,
    avg5: f64,
    avg15: f64,
    current_running_processes: usize,
    total_processes: usize,
    recent_pid: usize,
}

impl LoadAvg {
    fn parse(content: &str) -> Result<Self, LoadAvgError> {
        let mut split: Vec<&str> = content.split([' ', '/']).collect();
        Ok(Self {
            avg1: split.remove(0).parse()?,
            avg5: split.remove(0).parse()?,
            avg15: split.remove(0).parse()?,
            current_running_processes: split.remove(0).parse()?,
            total_processes: split.remove(0).parse()?,
            recent_pid: split.remove(0).trim().parse()?,
        })
    }
}

pub(crate) struct LoadAvgFile {
    path: String,
}

#[async_trait]
impl File for LoadAvgFile {
    type Output = LoadAvg;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        system.read_to_string(self.path())
            .await
            .map(|s| LoadAvg::parse(s.as_str()))?.map_err(Into::into)
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoadAvgBuilder;

impl FileBuilder for LoadAvgBuilder {
    type File = LoadAvgFile;

    const NAME: &'static str = "loadavg";
    const DESCRIPTION: &'static str = "Get load average";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/loadavg", &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    LoadAvg {
                        avg1: 0.15,
                        avg5: 1.53,
                        avg15: 2.52,
                        recent_pid: 12345,
                        total_processes: 54363,
                        current_running_processes: 1
                    }
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[derive(Debug, Error)]
pub(crate) enum LoadAvgError {
    #[error("failed to parse {0}")]
    ParseInt(ParseIntError),
    #[error("failed to parse {0}")]
    ParseFloat(ParseFloatError),
}

impl From<ParseIntError> for LoadAvgError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

impl From<ParseFloatError> for LoadAvgError {
    fn from(value: ParseFloatError) -> Self {
        Self::ParseFloat(value)
    }
}

#[cfg(test)]
mod test {
    use crate::files::loadavg::LoadAvg;
    use crate::utils::test::read_test_resources;

    #[test]
    pub(crate) fn test_parse() {
        assert_eq!(LoadAvg::parse(read_test_resources("loadavg").as_str()).unwrap(),
                   LoadAvg {
                       avg1: 0.07,
                       avg5: 0.42,
                       avg15: 0.55,
                       current_running_processes: 1,
                       total_processes: 820,
                       recent_pid: 19277,
                   });
    }
}