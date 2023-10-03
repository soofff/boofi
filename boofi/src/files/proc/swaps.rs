use crate::files::prelude::*;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct Swap {
    filename: String,
    r#type: String,
    size: usize,
    used: bool,
    priority: isize,
}

impl Swap {
    pub(crate) fn parse(content: &str) -> Resul<Vec<Swap>> {
        content.split('\n')
            .filter_map(|line| {
                let l = line.trim();
                if !l.is_empty() && !l.contains("Filename") {
                    let mut s: Vec<&str> = l.split([' ', '\t'])
                        .filter(|s| !s.is_empty())
                        .collect();
                    Some((|| -> Resul<Self> {
                        dbg!(&s);
                        Ok(Self {
                            filename: s.remove(0).into(),
                            r#type: s.remove(0).into(),
                            size: s.remove(0).parse()?,
                            used: s.remove(0) == "1",
                            priority: s.remove(0).parse()?,
                        })
                    })())
                } else {
                    None
                }
            }).collect()
    }
}

pub(crate) struct SwapsFile {
    path: String,
}

#[async_trait]
impl File for SwapsFile {
    type Output = Vec<Swap>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Swap::parse(&system.read_to_string(self.path()).await?)
    }

    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct SwapsBuilder;

impl FileBuilder for SwapsBuilder {
    type File = SwapsFile;

    const NAME: &'static str = "swaps";
    const DESCRIPTION: &'static str = "Swap information";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/swaps", &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    vec![Swap {
                            size: 1234,
                            filename: "/swap".into(),
                            used: false,
                            priority: -2,
                            r#type: "file".into()
                       }]
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::swaps::Swap;
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        assert_eq!(Swap::parse(&read_test_resources("swaps")).unwrap(), vec![
            Swap { filename: "/swapfile".into(), r#type: "file".into(), size: 2097148, used: false, priority: -2 }
        ]);
    }
}