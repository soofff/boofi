use crate::files::prelude::*;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct Mounts {
    device: String,
    target: String,
    filesystem: String,
    options: Vec<String>,
    dump: usize,
    fsck: usize,
}

impl Mounts {
    pub(crate) fn parse(content: &str) -> Resul<Vec<Self>> {
        content.trim()
            .split('\n')
            .map(|line| {
                let mut s: Vec<&str> = line.split(' ')
                    .filter(|s| !s.is_empty())
                    .collect();

                (|| -> Resul<Self> {
                    Ok(Self {
                        device: s.remove(0).into(),
                        target: s.remove(0).into(),
                        filesystem: s.remove(0).into(),
                        options: s.remove(0).split(',').map(ToString::to_string).collect(),
                        dump: s.remove(0).parse()?,
                        fsck: s.remove(0).parse()?,
                    })
                })()
            })
            .collect::<Resul<Vec<Self>>>()
            .map_err(Into::into)
    }
}


pub(crate) struct MountsFile {
    path: String,
}

#[async_trait]
impl File for MountsFile {
    type Output = Vec<Mounts>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Mounts::parse(&system.read_to_string(self.path()).await?)
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct MountsBuilder;

impl FileBuilder for MountsBuilder {
    type File = MountsFile;

    const NAME: &'static str = "mounts";
    const DESCRIPTION: &'static str = "Mount information";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/mounts" ,  &[Os::LinuxAny])];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EAMPLES: [FileExample;1] = [
                FileExample::new_get("Simple example",
                    vec![Mounts {
                            dump: 0,
                            fsck: 0,
                            device: "/dev/sda1".into(),
                            options: vec!["rw".into()],
                            filesystem: "ext4".into(),
                            target: "/".into()
                       }]
                )
            ];
        }

        EAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::mounts::Mounts;
    use crate::utils::test::read_test_resources;

    #[tokio::test]
    async fn test_parse() {
        assert_eq!(Mounts::parse(&read_test_resources("mounts")).unwrap(),
                   vec![
                       Mounts {
                           device: "proc".into(),
                           target: "/proc".into(),
                           filesystem: "proc".into(),
                           options: vec!["rw", "nosuid", "nodev", "noexec", "relatime"].iter().map(ToString::to_string).collect(),
                           dump: 0,
                           fsck: 0,
                       },
                       Mounts {
                           device: "/dev/sda5".into(),
                           target: "/".into(),
                           filesystem: "ext4".into(),
                           options: vec!["rw", "relatime", "errors=remount-ro"].iter().map(ToString::to_string).collect(),
                           dump: 1,
                           fsck: 2,
                       },
                       Mounts {
                           device: "/dev/loop0".into(),
                           target: "/snap/core20/1974".into(),
                           filesystem: "squashfs".into(),
                           options: vec!["ro", "nodev", "relatime", "errors=continue"].iter().map(ToString::to_string).collect(),
                           dump: 0,
                           fsck: 0,
                       },
                       Mounts {
                           device: "/dev/fuse".into(),
                           target: "/run/user/1000/doc".into(),
                           filesystem: "fuse".into(),
                           options: vec!["rw", "nosuid", "nodev", "relatime", "user_id=1000", "group_id=1000"].iter().map(ToString::to_string).collect(),
                           dump: 0,
                           fsck: 0,
                       },
                   ]
        )
    }
}