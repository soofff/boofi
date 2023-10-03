use crate::files::prelude::*;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct Partition {
    major: usize,
    minor: usize,
    blocks: usize,
    name: String,
}

impl Partition {
    pub(crate) fn parse(content: &str) -> Resul<Vec<Self>> {
        content.split('\n')
            .filter_map(|line| {
                let l = line.trim();
                if !l.is_empty() && !l.contains("#blocks") {
                    let mut s: Vec<&str> = l.split([' ', '\t'])
                        .filter(|s| !s.is_empty())
                        .collect();
                    Some((|| -> Resul<Self> {
                        dbg!(&s);
                        Ok(Self {
                            major: s.remove(0).parse()?,
                            minor: s.remove(0).parse()?,
                            blocks: s.remove(0).parse()?,
                            name: s.remove(0).into(),
                        })
                    })())
                } else {
                    None
                }
            }).collect()
    }
}


pub(crate) struct PartitionsFile {
    path: String,
}

#[async_trait]
impl File for PartitionsFile {
    type Output = Vec<Partition>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Partition::parse(&system.read_to_string(self.path()).await?)
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct PartitionsBuilder;

impl FileBuilder for PartitionsBuilder {
    type File = PartitionsFile;

    const NAME: &'static str = "partitions";
    const DESCRIPTION: &'static str = "Partition information";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/partitions", &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    vec![Partition {
                            blocks: 4567,
                            major: 1,
                            minor: 2,
                            name: "sda1".into(),
                       }]
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::partitions::Partition;
    use crate::utils::test::read_test_resources;

    #[tokio::test]
    async fn test_parse() {
        assert_eq!(Partition::parse(&read_test_resources("partitions")).unwrap(), vec![
            Partition { major: 7, minor: 0, blocks: 64972, name: "loop0".into() },
            Partition { major: 11, minor: 0, blocks: 1048575, name: "sr0".into() },
            Partition { major: 8, minor: 0, blocks: 314572800, name: "sda".into() },
            Partition { major: 8, minor: 1, blocks: 524288, name: "sda1".into() },
            Partition { major: 8, minor: 2, blocks: 1, name: "sda2".into() },
            Partition { major: 8, minor: 5, blocks: 314045440, name: "sda5".into() },
        ]);
    }
}