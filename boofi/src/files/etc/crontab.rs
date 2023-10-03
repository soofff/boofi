use crate::files::prelude::*;
use std::mem::take;
use log::error;
use regex::Regex;
use thiserror::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize, Description)]
pub(crate) enum CrontabConfig {
    Shell(String),
    Path(String),
}

impl ToString for CrontabConfig {
    fn to_string(&self) -> String {
        match self {
            CrontabConfig::Shell(v) => format!("SHELL={}", v),
            CrontabConfig::Path(v) => format!("PATH={}", v)
        }
    }
}

impl CrontabConfig {
    fn parse(value: &str) -> Resul<Self> {
        if value.starts_with("SHELL") {
            Ok(Self::Shell(value.split_once('=').unwrap_or_default().1.into()))
        } else if value.starts_with("PATH") {
            Ok(Self::Path(value.split_once('=').unwrap_or_default().1.into()))
        } else {
            Err(CrontabError::UnknownConfig.into())
        }
    }
}

#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
pub(crate) struct CrontabJobValue {
    value: String,
    whitespaces: String,
}

impl ToString for CrontabJobValue {
    fn to_string(&self) -> String {
        format!("{}{}", self.value, self.whitespaces)
    }
}

impl CrontabJobValue {
    fn entire_len(&self) -> usize {
        self.value.len() + self.whitespaces.len()
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct CrontabJob {
    minute: CrontabJobValue,
    hour: CrontabJobValue,
    day_of_month: CrontabJobValue,
    month: CrontabJobValue,
    day_of_week: CrontabJobValue,
    user: CrontabJobValue,
    command: String,
}

impl ToString for CrontabJob {
    fn to_string(&self) -> String {
        format!("{minute}{hour}{day_of_month}{month}{day_of_week}{user}{command}",
                minute = self.minute.to_string(),
                hour = self.hour.to_string(),
                day_of_month = self.day_of_month.to_string(),
                month = self.month.to_string(),
                day_of_week = self.day_of_week.to_string(),
                user = self.user.to_string(),
                command = self.command
        )
    }
}

impl CrontabJob {
    pub(crate) fn parse(line: &str) -> Resul<Self> {
        let mut l = vec![];
        let mut v = CrontabJobValue::default();

        let mut last_empty = false;

        for c in line.chars() {
            if c == ' ' || c == '\t' {
                last_empty = true;
                v.whitespaces.push(c);
            } else {
                if last_empty {
                    // column complete
                    l.push(take(&mut v));

                    if l.len() == 6 {
                        // command column
                        break;
                    }
                }
                v.value.push(c);
                last_empty = false;
            }
        }

        if l.len() < 6 {
            return Err(CrontabError::TaskParse.into());
        }

        let offset: usize = l.iter().map(CrontabJobValue::entire_len).sum();

        Ok(Self {
            minute: l.remove(0),
            hour: l.remove(0),
            day_of_month: l.remove(0),
            month: l.remove(0),
            day_of_week: l.remove(0),
            user: l.remove(0),
            command: line[offset..].into(),
        })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Serialize, Deserialize, Description)]
pub(crate) enum CrontabLine {
    Comment(String),
    Linebreak,
    Config(CrontabConfig),
    Job(CrontabJob),
}

impl ToString for CrontabLine {
    fn to_string(&self) -> String {
        match self {
            CrontabLine::Comment(v) => v.to_string(),
            CrontabLine::Linebreak => "\n".to_string(),
            CrontabLine::Config(v) => v.to_string(),
            CrontabLine::Job(v) => v.to_string(),
        }
    }
}

impl CrontabLine {
    fn parse(value: &str) -> Resul<Self> {
        if value.is_empty() {
            return Ok(Self::Linebreak);
        } else if value.starts_with('#') {
            return Ok(Self::Comment(value.to_string()));
        }

        match CrontabConfig::parse(value) {
            Ok(c) => { Ok(Self::Config(c)) }
            Err(_) => { Ok(Self::Job(CrontabJob::parse(value)?)) }
        }
    }

    fn is_linebreak(&self) -> bool {
        matches!(self, CrontabLine::Linebreak)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Description)]
pub(crate) struct Crontab {
    content: Vec<CrontabLine>,
}

impl ToString for Crontab {
    fn to_string(&self) -> String {
        let r: String = self.content.iter().enumerate().filter_map(|(i, l)| {
            if i == self.content.len() - 1 && l == &CrontabLine::Linebreak {
                // skip linebreak if last crontab line is linebreak because it would create double \n
                return None;
            }

            let mut s = l.to_string();
            if !l.is_linebreak() {
                // append linebreak except linebreak itself
                s.push('\n');
            }
            Some(s)
        }).collect();
        r
    }
}

impl Crontab {
    pub(crate) fn parse(content: &str) -> Resul<Self> {
        content.split('\n')
            .map(CrontabLine::parse)
            .collect::<Resul<Vec<CrontabLine>>>()
            .map(|lines| {
                Self {
                    content: lines
                }
            })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CrontabBuilder;

impl FileBuilder for CrontabBuilder {
    file_metadata!(
        CrontabFile,
        "cronjob",
        "read and write cronjob file",
        &[Capability::Read, Capability::Write, Capability::Delete],
        FileExample::new_get("read crontab",
            vec![
                CrontabLine::Comment("# /etc/crontab: system-wide crontab".into()),
                CrontabLine::Linebreak, CrontabLine::Config(CrontabConfig::Shell("/bin/sh".into())),
                CrontabLine::Config(CrontabConfig::Path("/usr/local/sbin:/usr/local/bin:/sbin:/bin:/usr/sbin:/usr/bin".into())),
                CrontabLine::Linebreak,
                CrontabLine::Comment("# Jobs".into()),
                CrontabLine::Job(CrontabJob {
                    minute: CrontabJobValue { value: "17".into(), whitespaces: " ".into() },
                    hour: CrontabJobValue { value: "*".into(), whitespaces: "	".into() },
                    day_of_month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    day_of_week: CrontabJobValue { value: "*".into(), whitespaces: "	".into() },
                    user: CrontabJobValue { value: "root".into(), whitespaces: "    ".into() },
                    command: "cd / && run-parts --report /etc/cron.hourly".into()
                })
            ]
        )
        ;
        FileMatchPattern::new_path("/etc/crontab", &[Os:: LinuxAny]),
        FileMatchPattern::new_regex(Regex::new("/etc/cron\\.d/.*").unwrap(), &[Os::LinuxAny])
    );
}

pub(crate) struct CrontabFile {
    path: String,
}

#[async_trait]
impl File for CrontabFile {
    type Output = Crontab;
    type Input = Crontab;

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Crontab::parse(&system.read_to_string(self.path()).await?)
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let i = Crontab::deserialize(input).map_err(Erro::from_deserialize)?;
        system.write(self.path(), i.to_string().as_bytes()).await
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Error)]
pub(crate) enum CrontabError {
    #[error("unknown crontab config variable")]
    UnknownConfig,
    #[error("failed to parse task")]
    TaskParse,
}

#[cfg(test)]
mod test {
    use crate::files::crontab::{Crontab, CrontabConfig, CrontabJob, CrontabJobValue};
    use crate::files::crontab::CrontabLine::{Comment, Config, Job, Linebreak};
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse_and_string() {
        let cronjob = Crontab {
            content: vec![
                Comment("# /etc/crontab: system-wide crontab".into()),
                Linebreak, Config(CrontabConfig::Shell("/bin/sh".into())),
                Config(CrontabConfig::Path("/usr/local/sbin:/usr/local/bin:/sbin:/bin:/usr/sbin:/usr/bin".into())),
                Linebreak,
                Comment("# Jobs".into()),
                Job(CrontabJob {
                    minute: CrontabJobValue { value: "17".into(), whitespaces: " ".into() },
                    hour: CrontabJobValue { value: "*".into(), whitespaces: "	".into() },
                    day_of_month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    day_of_week: CrontabJobValue { value: "*".into(), whitespaces: "	".into() },
                    user: CrontabJobValue { value: "root".into(), whitespaces: "    ".into() },
                    command: "cd / && run-parts --report /etc/cron.hourly".into(),
                }),
                Job(CrontabJob {
                    minute: CrontabJobValue { value: "25".into(), whitespaces: " ".into() },
                    hour: CrontabJobValue { value: "6".into(), whitespaces: "	".into() },
                    day_of_month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    month: CrontabJobValue { value: "*".into(), whitespaces: " ".into() },
                    day_of_week: CrontabJobValue { value: "*".into(), whitespaces: "	".into() },
                    user: CrontabJobValue { value: "root".into(), whitespaces: "	".into() },
                    command: "test -x /usr/sbin/anacron || ( cd / && run-parts --report /etc/cron.daily )".into(),
                }),
                Linebreak,
            ],
        };

        let cronjob_string = read_test_resources("crontab");

        assert_eq!(Crontab::parse(&cronjob_string).unwrap(), cronjob);
        assert_eq!(cronjob.to_string(), cronjob_string);
    }
}
