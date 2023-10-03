use std::vec;
use serde::{Deserializer};
use boofi_macros::Description;
use crate::apps::prelude::*;
use crate::system::os::Os;
use crate::system::System;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) enum LsArguments {
    All,
    List,
    HumanReadable,
    File(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Description)]
pub(crate) struct LsEntry {
    filename: String,
    size: Option::<String>,
    permissions: Option::<String>,
}

impl LsEntry {
    pub(crate) fn filename(&self) -> &str { self.filename.as_str() }
    pub(crate) fn size(&self) -> Option<&str> { self.size.as_deref() }

    pub(crate) fn parse_from_line(arguments: &LsInput, line: &str) -> Resul<Self> {
        let (permissions,
            size,
            filename,
        ) = if arguments.list == Some(true) {
            let parts: Vec<&str> = line.split_whitespace().filter(|s| {
                !s.is_empty()
            }).collect();

            (Some(parts[0].to_string()),
             Some(parts[4].to_string()),
             parts[8..].join(" "))
        } else {
            (None, None, line.to_string())
        };

        Ok(Self {
            filename,
            size,
            permissions,
        })
    }
}


#[derive(Serialize, Deserialize, Debug, Description)]
pub(crate) struct LsInput {
    list: Option::<bool>,
    all: Option::<bool>,
    human_readable: Option::<bool>,
    classify: Option::<bool>,
    path: String,
}

impl LsInput {
    pub(crate) fn new<T, P>(list: T,
                            all: T,
                            human_readable: T,
                            classify: T,
                            path: P,
    ) -> Self where
        T: Into<Option<bool>>,
        P: Into<String>,
    {
        Self {
            list: list.into(),
            all: all.into(),
            human_readable: human_readable.into(),
            classify: classify.into(),
            path: path.into(),
        }
    }
}

pub(crate) struct Ls;

impl Ls {
    pub(crate) fn parse(input: &LsInput, content: &str) -> Resul<Vec<LsEntry>> {
        content.split('\n')
            .skip(1)// skip "total .."
            .filter(|s| !s.is_empty())
            .map(|line| LsEntry::parse_from_line(input, line))
            .collect::<Resul<Vec<LsEntry>>>()
            .map_err(Into::into)
    }
}

pub(crate) struct LsApp {}

impl LsApp {
    pub(crate) async fn run_parse(input: LsInput, system: &System) -> Resul<Vec<LsEntry>> {
        let mut arguments = vec![];

        if input.all == Some(true) { arguments.push("-a") }
        if input.list == Some(true) { arguments.push("-l") }
        if input.human_readable == Some(true) { arguments.push("-h") }
        if input.classify == Some(true) { arguments.push("-F") }

        arguments.push(input.path.as_str());

        Ls::parse(&input,
                  &String::from_utf8(
                      system.run_args(LsBuilder::path(), arguments.as_slice()).await?,
                  )?,
        )
    }
}

#[derive(Clone)]
#[derive(Default)]
pub(crate) struct LsBuilder {}

impl LsBuilder {
    fn path() -> &'static str { "/bin/ls" }
}

#[async_trait]
impl App for LsApp {
    type Output = Vec<LsEntry>;
    type Input = LsInput;

    fn new() -> Self {
        Self {}
    }

    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, input: I, system: &System) -> Resul<Self::Output> {
        let ls_input = LsInput::deserialize(input).map_err(Erro::from_deserialize)?;
        LsApp::run_parse(ls_input, system).await
    }
}

impl AppBuilder for LsBuilder {
    type App = LsApp;

    const NAME: &'static str = "ls";
    const DESCRIPTION: &'static str = "Use ls to list directory and files.";
    const SUPPORTED_OS: &'static [Os] = &[Os::LinuxAny];

    fn examples(&self) -> &[AppExample] {
        lazy_static! {
            static ref EXAMPLE: [AppExample; 1] = [
                AppExample::new(
                    "Show files human readable with details.",
                    Box::new(LsInput {
                        list: Some(true),
                        all: Some(false),
                        human_readable: Some(true),
                        classify: None,
                        path: "/etc".into()
                    }),
                    Box::new(vec![LsEntry {
                        filename: "database.db".to_string(),
                        size: Some("1235 Mb".to_string()),
                        permissions: Some("rw-------".to_string()),
                    }])
                )
            ];
        }
        EXAMPLE.as_slice()
    }
}


#[cfg(test)]
mod test {
    use crate::apps::ls::{LsInput, Ls, LsEntry};
    use crate::utils::test::{read_test_resources};

    #[test]
    fn test_parse() {
        assert_eq!(Ls::parse(
            &LsInput {
                list: Some(true),
                all: Some(true),
                human_readable: None,
                classify: None,
                path: "/boot".into(),
            }, &read_test_resources("ls_la")).unwrap(), [
                       LsEntry {
                           filename: "config-5.15.0-78-generic".into(),
                           size: Some(
                               "262224".into(),
                           ),
                           permissions: Some(
                               "-rw-r--r--".into(),
                           ),
                       },
                       LsEntry {
                           filename: "grub".into(),
                           size: Some(
                               "4096".into(),
                           ),
                           permissions: Some(
                               "drwxr-xr-x".into(),
                           ),
                       },
                       LsEntry {
                           filename: "initrd.img-5.15.0-78-generic".into(),
                           size: Some(
                               "73928341".into(),
                           ),
                           permissions: Some(
                               "-rw-r--r--".into(),
                           ),
                       },
                       LsEntry {
                           filename: "vmlinuz -> vmlinuz-5.15.0-78-generic".into(),
                           size: Some(
                               "25".into(),
                           ),
                           permissions: Some(
                               "lrwxrwxrwx".into(),
                           ),
                       },
                   ]);
    }
}