use crate::files::prelude::*;
use crate::files::Regex;

#[derive(Debug)]
pub(crate) struct Text {
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TextCreateInput {
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Description)]
pub(crate) struct TextInput {
    content: String,
}

#[async_trait]
impl File for Text {
    type Output = String;
    type Input = TextInput;

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        system.read_to_string(self.path.as_str()).await
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let i = TextInput::deserialize(input).map_err(Erro::from_deserialize)?;
        system.write(self.path.as_str(), i.content.as_str().as_bytes()).await
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TextBuilder;

impl FileBuilder for TextBuilder {
    type File = Text;

    const NAME: &'static str = "text";
    const DESCRIPTION: &'static str = "Get text files, create new text file, replace content or append it.";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read, Capability::Write, Capability::Delete];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_regex(Regex::new(".*").unwrap(), &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLE: [FileExample;2] = [
                FileExample::new_get("Text file", "Some text \nAnd more\nAnd end"),
                FileExample::new_write("Create new text file", TextCreateInput {
                    content: "A new Text file\nHave a good day.".to_string()
                }),
            ];
        }

        EXAMPLE.as_slice()
    }
}
