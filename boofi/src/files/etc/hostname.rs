use crate::files::hosts::HostsManaged;
use crate::files::prelude::*;

pub(crate) struct Hostname {
    path: String,
}

#[derive(Deserialize, Description)]
pub(crate) struct HostnameInput {
    hostname: String,
}

#[async_trait]
impl File for Hostname {
    type Output = String;
    type Input = HostnameInput;

    fn new(path: &str) -> Self {
        Self { path: path.into() }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        system.read_to_string(self.path()).await
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let i = HostnameInput::deserialize(input).map_err(Erro::from_deserialize)?;
        system.write(self.path(), i.hostname.as_bytes()).await
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct HostnameBuilder;

impl FileBuilder for HostnameBuilder {
    type File = HostsManaged;

    const NAME: &'static str = "hostname";
    const DESCRIPTION: &'static str = "Get or set hostname";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read, Capability::Write, Capability::Delete];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/etc/hostname", &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLES: Vec<FileExample> = vec![
                FileExample::new_get("Hostname", "linux386")
            ];
        }

        EXAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use tokio::fs::read_to_string;
    use crate::files::File;
    use crate::files::hostname::{Hostname};
    use crate::utils::test::{system_user, test_resources};

    #[tokio::test]
    async fn test_parse_and_string() {
        let hostname_path = test_resources("hostname");
        let hostname_string = read_to_string(&hostname_path).await.unwrap();

        let mut hostname = Hostname {
            path: hostname_path.clone(),
        };

        assert_eq!(hostname.read(&system_user().await).await.unwrap(), json!(hostname_string));

        hostname.path = "/tmp/hostname.tmp".into();
        hostname.write(json!({ "hostname": hostname_string.clone() }), &system_user().await).await.unwrap();

        assert_eq!(read_to_string(&hostname.path).await.unwrap(), hostname_string);
    }
}