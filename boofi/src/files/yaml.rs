use serde_yaml::{from_slice, to_string, Value};
use crate::files::prelude::*;
use crate::files::Regex;

#[derive(Debug)]
pub(crate) struct Yaml {
    path: String,
}

impl Description for Value {
    const DESCRIPTION: &'static str = "yaml data";
}

#[async_trait]
impl File for Yaml {
    type Output = Value;
    type Input = Value;

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        from_slice(&system.read(self.path()).await?).map_err(Into::into)
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let value = Value::deserialize(input).map_err(Erro::from_deserialize)?;
        system.write(self.path(), to_string(&value)?.as_bytes()).await
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub(crate) struct YamlBuilder;

impl FileBuilder for YamlBuilder {
    type File = Yaml;

    const NAME: &'static str = "yaml";
    const DESCRIPTION: &'static str = "Read or write yaml file";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read, Capability::Write, Capability::Delete];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_regex(Regex::new("^.*.(yaml|YAML|yml|YML)$").unwrap(), &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLE: [FileExample;2] = [
                FileExample::new_get("simple yaml", "hello: world"),
                FileExample::new_write("simple yaml", "hello: world"),
            ];
        }

        EXAMPLE.as_slice()
    }
}


#[cfg(test)]
mod test {
    use serde_yaml::to_string;
    use tokio::fs::read_to_string;
    use crate::files::File;
    use crate::files::yaml::Yaml;
    use crate::utils::test::system_user;

    #[tokio::test]
    async fn test_write_and_read() {
        let path = "/tmp/_yaml_test_file";
        let y = Yaml::new(path);

        let system = system_user().await;
        y.write(serde_json::json!({
            "a": 1,
            "b": "2"
        }), &system).await.unwrap();

        let file = read_to_string(path).await.unwrap();
        let s = y.read(&system).await.unwrap();

        assert_eq!(&file, &to_string(&s).unwrap());

        y.delete(&system).await.unwrap();
    }
}