use crate::files::prelude::*;

#[derive(Serialize, Debug, PartialEq, Description)]
pub(crate) struct Uptime {
    uptime: f64,
    idle: f64,
}

impl Uptime {
    pub(crate) fn parse(content: &str) -> Resul<Self> {
        let mut s: Vec<&str> = content.trim().split(' ').collect();

        Ok(Self {
            uptime: s.remove(0).parse()?,
            idle: s.remove(0).parse()?,
        })
    }
}

pub(crate) struct UptimeFile {
    path: String,
}

#[async_trait]
impl File for UptimeFile {
    type Output = Uptime;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Uptime::parse(&system.read_to_string(self.path()).await?)
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct UptimeBuilder;

impl FileBuilder for UptimeBuilder {
    type File = UptimeFile;

    const NAME: &'static str = "uptime";
    const DESCRIPTION: &'static str = "Get uptime and idle time or each cpu (total) in seconds";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/uptime", &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    Uptime {
                        uptime: 123.45,
                        idle: 6789.0
                    }
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::uptime::Uptime;
    use crate::utils::test::read_test_resources;

    #[test]
    pub(crate) fn test_parse() {
        assert_eq!(Uptime::parse(read_test_resources("uptime").as_str()).unwrap(), Uptime {
            uptime: 874.22,
            idle: 2264.90,
        });
    }
}