use std::collections::HashMap;
use crate::files::prelude::*;
use thiserror::Error;


#[derive(Serialize, Debug, PartialEq, Description)]
pub(crate) struct OsRelease {
    name: String,
    version: Option<String>,
    id: String,
    id_like: Option<String>,
    version_id: Option<String>,
    pretty_name: Option<String>,
    ansi_color: Option<String>,
    cpe_name: Option<String>,
    build_id: Option<String>,
    home_url: Option<String>,
    bug_report_url: Option<String>,
    support_url: Option<String>,
    privacy_policy_url: Option<String>,
    variant: Option<String>,
    variant_id: Option<String>,
    version_codename: Option<String>,
}

impl OsRelease {
    pub(crate) fn id(&self) -> &str { self.id.as_str() }

    pub(crate) fn version_codename(&self) -> Option<&str> { self.version_codename.as_deref() }
}

impl TryFrom<String> for OsRelease {
    type Error = OsReleaseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut lines: HashMap<_, _> = value.split('\n')
            .filter_map(|s| {
                if s.is_empty() {
                    None
                } else {
                    s.split_once('=')
                        .map(|(k, v)| (k, v.to_string()))
                }
            })
            .collect();

        Ok(Self {
            name: lines.remove("NAME").ok_or(Self::Error::Name)?,
            version: lines.remove("VERSION"),
            id: lines.remove("ID").ok_or(Self::Error::Id)?,
            id_like: lines.remove("ID_LIKE"),
            version_id: lines.remove("VERSION_ID"),
            pretty_name: lines.remove("PRETTY_NAME"),
            ansi_color: lines.remove("ANSI_COLOR"),
            cpe_name: lines.remove("CPE_NAME"),
            build_id: lines.remove("BUILD_ID"),
            home_url: lines.remove("HOME_URL"),
            bug_report_url: lines.remove("BUG_REPORT_URL"),
            support_url: lines.remove("SUPPORT_URL"),
            privacy_policy_url: lines.remove("PRIVACY_POLICY_URL"),
            variant: lines.remove("VARIANT"),
            variant_id: lines.remove("VARIANT_ID"),
            version_codename: lines.remove("VERSION_CODENAME"),
        })
    }
}

pub(crate) struct OsReleaseFile {
    path: String,
}

impl OsReleaseFile {
    pub(crate) async fn release(&self, system: &System) -> Resul<OsRelease> {
        system.read_to_string(self.path.as_str())
            .await?
            .try_into()
            .map_err(Into::into)
    }
}

#[async_trait]
impl File for OsReleaseFile {
    type Output = OsRelease;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into()
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        self.release(system).await
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct OsReleaseBuilder;

impl FileBuilder for OsReleaseBuilder {
    type File = OsReleaseFile;

    const NAME: &'static str = "os-release";
    const DESCRIPTION: &'static str = "read os-release file";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/etc/os-release", &[Os::LinuxUbuntu])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLES: Vec<FileExample> = vec![];
        }

        EXAMPLES.as_slice()
    }
}

#[derive(Debug, Error)]
pub(crate) enum OsReleaseError {
    #[error("NAME missing")]
    Name,
    #[error("ID missing")]
    Id,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_parse() {}
}