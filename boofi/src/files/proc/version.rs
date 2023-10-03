use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct Version {
    version: String,
    compiled_by: String,
    compiled_host: String,
    compiler: String,
}

impl Version {
    pub(crate) fn parse(content: &str) -> Resul<Self> {
        let (version, s) = content.split_once(" (").ok_or(VersionError::Version)?;
        let (compiled_by, s) = s.split_once('@').ok_or(VersionError::CompiledBy)?;
        let (compiled_host, s) = s.split_once(") (").ok_or(VersionError::CompilerHost)?;
        let (compiler, _) = s.rsplit_once(") ").ok_or(VersionError::Compiler)?;

        Ok(Self {
            version: version.into(),
            compiled_by: compiled_by.into(),
            compiled_host: compiled_host.into(),
            compiler: compiler.into(),
        })
    }

    pub(crate) fn version(&self) -> &str { &self.version }
}

#[derive(Description)]
pub(crate) struct VersionFile {
    path: String,
}

#[async_trait]
impl File for VersionFile {
    type Output = Version;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        system.read_to_string(&self.path)
            .await
            .map(|s| Version::parse(s.as_str()))?
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct VersionBuilder;

impl FileBuilder for VersionBuilder {
    file_metadata!(
        VersionFile,
        "version",
        "Kernel version and compiler information",
        &[Capability::Read],
        FileExample::new_get("Simple example",
            Version {
                version: "Linux version 5.15.0-76-generic".into(),
                compiled_by: "buildd".into(),
                compiled_host: "lcy02-amd64-019".into(),
                compiler: "gcc (Ubuntu 9.4.0-1ubuntu1~20.04.1) 9.4.0, GNU ld (GNU Binutils for Ubuntu) 2.34".into(),
            }
        )
        ;
        FileMatchPattern::new_path("/proc/version", &[Os::LinuxAny])
    );
}

#[derive(Debug, Error)]
pub(crate) enum VersionError {
    #[error("failed to parse version")]
    Version,
    #[error("failed to parse compiled by")]
    CompiledBy,
    #[error("failed to parse compiler host")]
    CompilerHost,
    #[error("failed to parse compiler")]
    Compiler,
}

#[cfg(test)]
mod test {
    use crate::files::version::Version;
    use crate::utils::test::read_test_resources;

    #[test]
    pub(crate) fn test_parse() {
        assert_eq!(Version::parse(&read_test_resources("version")).unwrap(), Version {
            version: "Linux version 5.15.0-76-generic".into(),
            compiled_by: "buildd".into(),
            compiled_host: "lcy02-amd64-019".into(),
            compiler: "gcc (Ubuntu 9.4.0-1ubuntu1~20.04.1) 9.4.0, GNU ld (GNU Binutils for Ubuntu) 2.34".into(),
        });
    }
}