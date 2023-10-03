use std::fmt::Display;
use std::mem::take;
use crate::files::prelude::*;

#[derive(PartialEq, Debug, Serialize, Deserialize, Default, Description)]
pub(crate) struct FstabItem<T> {
    value: T,
    delimiter: String,
}

impl<T: Display> ToString for FstabItem<T> {
    fn to_string(&self) -> String {
        format!("{}{}", self.value, self.delimiter)
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Description)]
pub(crate) struct FstabEntry {
    device: FstabItem<String>,
    target: FstabItem<String>,
    filesystem: FstabItem<String>,
    options: FstabItem<Vec<String>>,
    dump: FstabItem<usize>,
    fsck: FstabItem<usize>,
}

impl ToString for FstabEntry {
    fn to_string(&self) -> String {
        format!("{}{}{}{}{}{}{}",
                self.device.to_string(),
                self.target.to_string(),
                self.filesystem.to_string(),
                self.options.value.join(","), self.options.delimiter,
                self.dump.to_string(),
                self.fsck.to_string(),
        )
    }
}

impl TryFrom<FstabItem<String>> for FstabItem<usize> {
    type Error = Erro;

    fn try_from(value: FstabItem<String>) -> Result<Self, Self::Error> {
        Ok(Self {
            value: value.value.parse()?,
            delimiter: value.delimiter,
        })
    }
}

impl TryFrom<FstabItem<String>> for FstabItem<Vec<String>> {
    type Error = Erro;

    fn try_from(value: FstabItem<String>) -> Result<Self, Self::Error> {
        Ok(Self {
            value: value.value.split(',').map(ToString::to_string).collect(),
            delimiter: value.delimiter,
        })
    }
}


impl FstabEntry {
    fn parse(line: &str) -> Resul<Self> {
        let mut items: Vec<FstabItem<String>> = vec![];
        let mut is_new = false;

        let mut item = FstabItem {
            value: Default::default(),
            delimiter: Default::default(),
        };

        for x in line.chars() {
            if x == ' ' || x == '\t' {
                is_new = true;
                item.delimiter.push(x)
            } else {
                if is_new {
                    items.push(take(&mut item));
                    is_new = false;
                }

                item.value.push(x)
            }
        }

        Ok(Self {
            device: items.remove(0),
            target: items.remove(0),
            filesystem: items.remove(0),
            options: items.remove(0).try_into()?,
            dump: items.remove(0).try_into()?,
            fsck: item.try_into()?,
        })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Debug, Serialize, Deserialize, Description)]
pub(crate) enum FstabLine {
    Comment(String),
    Empty,
    Entry(FstabEntry),
}

impl ToString for FstabLine {
    fn to_string(&self) -> String {
        match self {
            FstabLine::Comment(c) => c.into(),
            FstabLine::Empty => "".into(),
            FstabLine::Entry(e) => e.to_string()
        }
    }
}

impl FstabLine {
    fn parse(line: &str) -> Resul<Self> {
        Ok(if line.starts_with('#') {
            Self::Comment(line.into())
        } else if line.is_empty() {
            Self::Empty
        } else {
            Self::Entry(FstabEntry::parse(line)?)
        })
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Description)]
pub(crate) struct Fstab {
    content: Vec<FstabLine>,
}

impl Fstab {
    fn parse(content: &str) -> Resul<Self> {
        Ok(Self {
            content: content.split('\n')
                .map(FstabLine::parse)
                .collect::<Resul<_>>()?
        })
    }
}

impl ToString for Fstab {
    fn to_string(&self) -> String {
        self.content.iter().map(ToString::to_string).collect::<Vec<String>>().join("\n")
    }
}

pub(crate) struct FstabFile {
    path: String,
}

#[async_trait]
impl File for FstabFile {
    type Output = Fstab;
    type Input = Fstab;

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Fstab::parse(&system.read_to_string(self.path()).await?)
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let fstab = Fstab::deserialize(input).map_err(Erro::from_deserialize)?;
        system.write(self.path(), fstab.to_string().as_bytes()).await
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FstabBuilder;

impl FileBuilder for FstabBuilder {
    file_metadata!(
        FstabFile,
        "fstab",
        "Read and write fstab file. Modify behaves like create. In/output variables are equal.",
        &[Capability::Read, Capability::Write, Capability::Delete],
        FileExample::new_get("read fstab",
            Fstab { content: vec![
                FstabLine::Comment("# /etc/fstab: static file system information.".into()),
                FstabLine::Comment("#".into()),
                FstabLine::Comment("# <file system> <mount point>   <type>  <options>       <dump>  <pass>".into()),
                FstabLine::Entry(FstabEntry {
                    device: FstabItem { value: "UUID=33556600-c612-49a5-9e48-df1c531e9460".into(), delimiter: " ".into() },
                    target: FstabItem { value: "/".into(), delimiter: "               ".into() },
                    filesystem: FstabItem { value: "ext4".into(), delimiter: "    ".into() },
                    options: FstabItem { value: vec!["rw".into(), "user".into(), "noauto".into(), "uid=0".into(), "gid=46".into(), "umask=007".into(), "nls=utf8".into()], delimiter: "	 ".into() },
                    dump: FstabItem { value: 0, delimiter: "       ".into() },
                    fsck: FstabItem { value: 1, delimiter: "".into() }
                })
            ]}
        )
        ;
        FileMatchPattern::new_path("/etc/fstab", &[Os::LinuxAny])
    );
}

#[cfg(test)]
mod test {
    use crate::files::fstab::{Fstab, FstabEntry, FstabItem};
    use crate::files::fstab::FstabLine::{Comment, Empty, Entry};

    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        let content = read_test_resources("fstab");
        let fstab = Fstab {
            content: vec![
                Comment("# /etc/fstab: static file system information.".into()),
                Comment("#".into()),
                Comment("# <file system> <mount point>   <type>  <options>       <dump>  <pass>".into()),
                Entry(FstabEntry {
                    device: FstabItem { value: "UUID=33556600-c612-49a5-9e48-df1c531e9460".into(), delimiter: " ".into() },
                    target: FstabItem { value: "/".into(), delimiter: "               ".into() },
                    filesystem: FstabItem { value: "ext4".into(), delimiter: "    ".into() },
                    options: FstabItem { value: vec!["rw".into(), "user".into(), "noauto".into(), "uid=0".into(), "gid=46".into(), "umask=007".into(), "nls=utf8".into()], delimiter: "	 ".into() },
                    dump: FstabItem { value: 0, delimiter: "       ".into() },
                    fsck: FstabItem { value: 1, delimiter: "".into() },
                }),
                Comment("# another comment".into()),
                Entry(FstabEntry {
                    device: FstabItem { value: "UUID=B46E-3FC3".into(), delimiter: "  ".into() },
                    target: FstabItem { value: "/boot/efi".into(), delimiter: "       ".into() },
                    filesystem: FstabItem { value: "vfat".into(), delimiter: "    ".into() },
                    options: FstabItem { value: vec!["umask=0077".into()], delimiter: "      ".into() },
                    dump: FstabItem { value: 0, delimiter: "       ".into() },
                    fsck: FstabItem { value: 1, delimiter: "".into() },
                }),
                Entry(FstabEntry {
                    device: FstabItem { value: "/swapfile".into(), delimiter: "                                 ".into() },
                    target: FstabItem { value: "none".into(), delimiter: "            ".into() },
                    filesystem: FstabItem { value: "swap".into(), delimiter: "    ".into() },
                    options: FstabItem { value: vec!["sw".into()], delimiter: "              ".into() },
                    dump: FstabItem { value: 0, delimiter: "       ".into() },
                    fsck: FstabItem { value: 0, delimiter: "".into() },
                }),
                Empty,
            ]
        };

        assert_eq!(Fstab::parse(&content).unwrap(), fstab);
        assert_eq!(fstab.to_string(), content);
    }
}
