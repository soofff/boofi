use std::num::{ParseFloatError, ParseIntError};
use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct MdstatRecovery {
    progress: f32,
    progress_blocks: usize,
    finish: String,
    speed: String,
}

impl TryFrom<&str> for MdstatRecovery {
    type Error = MdstatError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut progress: Option<f32> = None;
        let mut progress_blocks: Option<usize> = None;
        let mut finish: Option<&str> = None;
        let mut speed: Option<&str> = None;

        let mut i = value.trim()
            .split([' ', '='].as_slice())
            .filter(|s| !s.is_empty());

        while let Some(s) = i.next() {
            if s == "recovery" {
                progress = i.next().map(|s| s[..s.len() - 1].parse()).transpose()?;
                progress_blocks = i.next().and_then(|s| {
                    s.split(['/', '(']).find(|s| !s.is_empty())
                        .map(|t| t.parse())
                }).transpose()?;
            }

            if s == "finish" {
                finish = i.next();
            }

            if s == "speed" {
                speed = i.next();
            }
        }

        Ok(Self {
            progress: progress.ok_or(MdstatError::RecoveryProgress)?,
            progress_blocks: progress_blocks.ok_or(MdstatError::RecoverySpeed)?,
            finish: finish.ok_or(MdstatError::RecoveryFinish)?.to_string(),
            speed: speed.ok_or(MdstatError::RecoverySpeed)?.to_string(),
        })
    }
}

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct MdstatDevice {
    name: String,
    number: usize,
    failed: bool,
}

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct MdstatItem {
    name: String,
    state: String,
    r#type: String,
    devices: Vec<MdstatDevice>,
    blocks: usize,
    recovery: Option<MdstatRecovery>,
}

impl TryFrom<String> for MdstatItem {
    type Error = MdstatError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut lines = value.split('\n').filter(|l| !l.is_empty());

        // first line
        let mut i = lines.next()
            .ok_or(MdstatError::MdUnknown)?
            .split([':', ' '].as_slice())
            .filter(|l| !l.is_empty());

        let name = i.next().ok_or(MdstatError::DeviceMdName)?;
        let state = i.next().ok_or(MdstatError::DeviceState)?;
        let level = i.next().ok_or(MdstatError::DeviceLevel)?;
        let devices = i.map(|item| {
            let mut a = item.split(['[', ']', '(', ')'].as_slice()).filter(|s| !s.is_empty());

            (move || -> Result<MdstatDevice, MdstatError> {
                Ok(MdstatDevice {
                    name: a.next().ok_or(MdstatError::DeviceName)?.to_string(),
                    number: a.next().ok_or(MdstatError::DeviceNumber)?.parse()?,
                    failed: a.next() == Some("F"),
                })
            })()
        }).collect::<Result<Vec<MdstatDevice>, MdstatError>>()?;

        // second line
        let ii: usize = lines.next()
            .ok_or(MdstatError::MdUnknown)?
            .split(' ').find(|s| !s.is_empty())
            .ok_or(MdstatError::BlocksMissing)?
            .parse()?;

        // third line
        let iii = lines.next().map(MdstatRecovery::try_from).transpose()?;

        Ok(Self {
            name: name.to_string(),
            state: state.to_string(),
            r#type: level.to_string(),
            devices,
            blocks: ii,
            recovery: iii,
        })
    }
}

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct MdstatDetails {
    personalities: Vec<String>,
    items: Vec<MdstatItem>,
}

pub(crate) struct Mdstat;

impl Mdstat {
    fn parse(content: &str) -> Resul<MdstatDetails> {
        let mut split = content.split_inclusive('\n');
        let personalities = split.next()
            .ok_or(MdstatError::Personalities)?
            .split(':')
            .last()
            .ok_or(MdstatError::Personalities)?
            .split([' ', '[', ']', '\n'].as_slice())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        let mut devices = vec![];

        let mut item = String::default();
        for d in split {
            if d.starts_with("md") && !item.is_empty() {
                devices.push(std::mem::take(&mut item));
            }
            item.push_str(d);
        }

        Ok(MdstatDetails {
            personalities,
            items: devices.into_iter()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<MdstatItem>, MdstatError>>()?,
        })
    }
}

pub(crate) struct MdstatFile {
    path: String,
}

#[async_trait]
impl File for MdstatFile {
    type Output = MdstatDetails;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Mdstat::parse(&system.read_to_string(&self.path).await?)
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct MdstatBuilder;

impl FileBuilder for MdstatBuilder {
    type File = MdstatFile;

    const NAME: &'static str = "mdstat";
    const DESCRIPTION: &'static str = "Get mdstat information.";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/proc/mdstat", &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLE: [FileExample;1] =  [FileExample::new_get("Single raid setup with recovery progress.",
                MdstatDetails {
                    personalities: vec!["raid0".to_string(), "raid1".to_string()],
                    items: vec![
                        MdstatItem {
                            name: "md0".to_string(),
                            state: "active".to_string(),
                            r#type: "raid1".to_string(),
                            devices: vec![MdstatDevice {
                                name: "sda".to_string(),
                                number: 0,
                                failed: false,
                            }, MdstatDevice {
                                name: "sdb".to_string(),
                                number: 2,
                                failed: false,
                            }],
                            blocks: 2353450,
                            recovery: Some(MdstatRecovery {
                                progress: 10.0,
                                progress_blocks: 235345,
                                finish: "42min".to_string(),
                                speed: "100Kb/s".to_string(),
                            }),
                        }
                    ],
                }
            )];
        }

        EXAMPLE.as_slice()
    }
}


#[derive(Debug, Error)]
pub(crate) enum MdstatError {
    #[error("failed to parse recovery progress")]
    RecoveryProgress,
    #[error("failed to parse recovery finish")]
    RecoveryFinish,
    #[error("failed to parse recovery speed")]
    RecoverySpeed,
    #[error("parse float: {0}")]
    ParseFloat(#[from] ParseFloatError),
    #[error("parse int: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("failed to parse")]
    MdUnknown,
    #[error("failed to parse device name")]
    DeviceMdName,
    #[error("failed to parse device state")]
    DeviceState,
    #[error("failed to parse device level")]
    DeviceLevel,
    #[error("failed to parse device name")]
    DeviceName,
    #[error("failed to parse device number")]
    DeviceNumber,
    #[error("failed to parse blocks")]
    BlocksMissing,
    #[error("failed to parse personalities")]
    Personalities,
}

#[cfg(test)]
mod test {
    use crate::files::mdstat::{Mdstat, MdstatDetails, MdstatDevice, MdstatItem, MdstatRecovery};
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        assert_eq!(Mdstat::parse(&read_test_resources("mdstat")).unwrap(),
                   MdstatDetails {
                       personalities: vec!["linear".into(), "raid0".into(), "raid1".into(), "raid10".into(), "raid6".into(), "raid5".into(), "raid4".into()],
                       items: vec![
                           MdstatItem {
                               name: "md3".into(),
                               state: "active".into(),
                               r#type: "raid1".into(),
                               devices: vec![
                                   MdstatDevice { name: "sdb1".into(), number: 1, failed: true },
                                   MdstatDevice { name: "sda1".into(), number: 0, failed: false }],
                               blocks: 104320,
                               recovery: None,
                           },
                           MdstatItem {
                               name: "md2".into(),
                               state: "active".into(),
                               r#type: "raid5".into(),
                               devices: vec![
                                   MdstatDevice { name: "hdc3".into(), number: 0, failed: false },
                                   MdstatDevice { name: "hde3".into(), number: 1, failed: false },
                                   MdstatDevice { name: "hdg3".into(), number: 2, failed: false }],
                               blocks: 112639744,
                               recovery: None,
                           },
                           MdstatItem {
                               name: "md1".into(),
                               state: "active".into(),
                               r#type: "raid1".into(),
                               devices: vec![
                                   MdstatDevice { name: "sdb3".into(), number: 2, failed: false },
                                   MdstatDevice { name: "sda3".into(), number: 0, failed: false }],
                               blocks: 3068288,
                               recovery: Some(MdstatRecovery {
                                   progress: 8.1,
                                   progress_blocks: 251596,
                                   finish: "6.7min".into(),
                                   speed: "6963K/sec".into(),
                               }),
                           }],
                   }
        );
    }
}