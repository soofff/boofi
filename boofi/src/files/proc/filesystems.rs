use crate::files::prelude::*;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct FilesystemItem {
    name: String,
    nodev: bool,
}

impl From<&str> for FilesystemItem {
    fn from(value: &str) -> Self {
        let mut i: Vec<&str> = value.split('\t').filter(|s| !s.is_empty()).collect();

        let (nodev, name) = if i.len() == 1 {
            (false, i.remove(0))
        } else {
            (true, i.remove(1))
        };

        Self {
            name: name.into(),
            nodev,
        }
    }
}

pub(crate) struct Filesystem;

impl Filesystem {
    async fn parse(content: &str) -> Vec<FilesystemItem> {
        content.split('\n')
            .filter(|s| !s.is_empty())
            .map(FilesystemItem::from)
            .collect()
    }
}

pub(crate) struct FilesystemFile {
    path: String,
}

#[async_trait]
impl File for FilesystemFile {
    type Output = Vec<FilesystemItem>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Ok(Filesystem::parse(&system.read_to_string(self.path()).await?).await)
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FilesystemBuilder;

impl FileBuilder for FilesystemBuilder {
    type File = FilesystemFile;

    const NAME: &'static str = "filesystems";
    const DESCRIPTION: &'static str = "Get filesystems";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern;1] = [FileMatchPattern::new_path("/proc/filesystems",
                &[Os::LinuxAny]
            )];
        }

        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLES: [FileExample;1] = [FileExample::new_get("supported filesystems",
                [
                                FilesystemItem { name: "sysfs".into(), nodev: true },
                FilesystemItem { name: "ext3".into(), nodev: false },
                FilesystemItem { name: "fuse".into(), nodev: true }
                ]
            )];
        }

        EXAMPLES.as_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::files::filesystems::{Filesystem, FilesystemItem};
    use crate::utils::test::read_test_resources;

    #[tokio::test]
    async fn test_parse() {
        assert_eq!(Filesystem::parse(&read_test_resources("filesystems")).await, vec![
            FilesystemItem { name: "sysfs".into(), nodev: true },
            FilesystemItem { name: "ext3".into(), nodev: false },
            FilesystemItem { name: "fuse".into(), nodev: true },
        ]);
    }
}
