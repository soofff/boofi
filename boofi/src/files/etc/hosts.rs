use std::str::FromStr;
use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Item {
    identifier: String,
    whitespaces: Option<String>,
}

impl ToString for Item {
    fn to_string(&self) -> String {
        format!("{}{}", self.identifier, self.whitespaces.as_ref().unwrap_or(&" ".to_string()))
    }
}

impl Default for Item {
    fn default() -> Self {
        Self {
            identifier: String::new(),
            whitespaces: Some(String::new()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Entry {
    address: Item,
    hosts: Vec<Item>,
}

impl ToString for Entry {
    fn to_string(&self) -> String {
        format!("{}{}", self.address.to_string(), self.hosts.iter().map(ToString::to_string).collect::<Vec<String>>().join(""))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Description)]
pub(crate) enum HostsLine {
    Comment(String),
    Entries(Entry),
    Empty,
}

impl ToString for HostsLine {
    fn to_string(&self) -> String {
        match self {
            HostsLine::Comment(s) => s.to_string(),
            HostsLine::Entries(s) => s.to_string(),
            HostsLine::Empty => "\n".to_string(),
        }
    }
}

impl FromStr for HostsLine {
    type Err = HostsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.is_empty() {
            Self::Empty
        } else if s.starts_with('#') {
            Self::Comment(s.to_string())
        } else {
            let mut entries = vec![];
            let mut item = Item::default();

            let mut new_item = false;

            for c in s.chars() {
                if c == '\t' || c == ' ' {
                    new_item = true;
                    item.whitespaces = item.whitespaces.map(|w| format!("{}{}", w, c));
                } else {
                    if new_item {
                        entries.push(std::mem::take(&mut item));
                        new_item = false;
                    }
                    item.identifier.push(c);
                }
            }

            if !s.is_empty() {
                entries.push(item);
            }

            Self::Entries(Entry {
                address: entries.remove(0),
                hosts: entries,
            })
        })
    }
}

#[derive(Debug)]
pub(crate) struct Hosts;

impl Hosts {
    fn parse(content: &str) -> Resul<Vec<HostsLine>> {
        content.lines().map(FromStr::from_str)
            .collect::<Result<Vec<HostsLine>, HostsError>>()
            .map(Into::into)
            .map_err(Into::into)
    }

    fn lines_to_string(lines: Vec<HostsLine>) -> String {
        lines.iter()
            .map(|host_line| {
                match host_line {
                    HostsLine::Comment(s) => s.to_owned() + "\n",
                    HostsLine::Entries(e) => e.to_string() + "\n",
                    HostsLine::Empty => HostsLine::Empty.to_string(),
                }
            })
            .collect::<Vec<String>>().join("")
    }
}


#[derive(Debug)]
pub(crate) struct HostsManaged {
    path: String,
}

impl HostsManaged {
    async fn parse(&self, system: &System) -> Resul<Vec<HostsLine>> {
        Hosts::parse(&system.read_to_string(&self.path).await?)
    }

    async fn write(&self, lines: Vec<HostsLine>, system: &System) -> Resul<()> {
        system.write(&self.path,
                     Hosts::lines_to_string(lines).as_bytes(),
        ).await.map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HostsInput {
    add: Option<Vec<HostsLine>>,
    remove: Option<Vec<String>>,
    overwrite: Option<bool>,
}

#[async_trait]
impl File for HostsManaged {
    type Output = Vec<HostsLine>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        self.parse(system).await
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let i = HostsInput::deserialize(input).map_err(Erro::from_deserialize)?;

        let mut c = if i.overwrite == Some(true) {
            vec![]
        } else {
            self.parse(system).await?
        };

        c.retain(|line| {
            if let HostsLine::Entries(entry) = line {
                if let Some(removes) = &i.remove {
                    return !removes.contains(&entry.address.identifier);
                }
            }
            true
        });

        if let Some(mut add) = i.add {
            c.append(&mut add);
        }

        self.write(c, system).await
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HostsBuilder {}

impl FileBuilder for HostsBuilder {
    type File = HostsManaged;

    const NAME: &'static str = "hosts";
    const DESCRIPTION: &'static str = "Manage hosts file. Preserve comments and whitespaces.";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read, Capability::Write, Capability::Delete];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/etc/hosts", &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
                static ref EXAMPLES: Vec<FileExample> = vec![
                FileExample::new_get("Some addresses with hosts.",   vec![
                        HostsLine::Comment("Example comment".to_string()),
                        HostsLine::Empty,
                        HostsLine::Entries(Entry {
                            address: Item {
                                identifier: "127.0.0.1".into(),
                                whitespaces: Some("\t".into())
                            },
                            hosts: vec![Item {
                                identifier: "localhost".into(),
                                whitespaces: None
                            },]
                        })
                    ]
                )
            ];
            }
        EXAMPLES.as_slice()
    }
}

#[derive(Debug, Error)]
pub(crate) enum HostsError {}

#[cfg(test)]
mod test {
    use crate::files::hosts::{Entry, Hosts, Item};
    use crate::files::hosts::HostsLine::{Comment, Entries, Empty};
    use crate::utils::test::read_test_resources;

    #[test]
    fn parse() {
        let entries = vec![
            Entries(
                Entry {
                    address: Item { identifier: "127.0.0.1".into(), whitespaces: Some("	".into()) },
                    hosts: vec![Item { identifier: "localhost".into(), whitespaces: Some("".into()) }],
                }),
            Empty,
            Comment("# The following lines are desirable for IPv6 capable hosts".into()),
            Entries(
                Entry {
                    address: Item { identifier: "::1".into(), whitespaces: Some("     ".into()) },
                    hosts: vec![Item { identifier: "ip6-localhost".into(), whitespaces: Some(" ".into()) },
                                Item { identifier: "ip6-loopback".into(), whitespaces: Some("".into()) }],
                }),
            Entries(
                Entry {
                    address: Item { identifier: "fe00::0".into(), whitespaces: Some(" ".into()) },
                    hosts: vec![Item { identifier: "ip6-localnet".into(), whitespaces: Some("".into()) }],
                }
            ),
        ];

        let content = read_test_resources("hosts");

        assert_eq!(Hosts::parse(&content).unwrap(), entries);
        assert_eq!(Hosts::lines_to_string(entries), content);
    }
}