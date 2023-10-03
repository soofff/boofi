pub(crate) mod text;
mod proc;
mod etc;
mod yaml;
mod json;

pub(crate) use proc::*;
pub(crate) use etc::*;

pub(crate) use crate::files::text::TextBuilder;
pub(crate) use crate::files::json::JsonBuilder;
pub(crate) use crate::files::yaml::YamlBuilder;
pub(crate) use crate::files::crontab::CrontabBuilder;
pub(crate) use crate::files::fstab::FstabBuilder;
pub(crate) use crate::files::hostname::HostnameBuilder;
pub(crate) use crate::files::hosts::HostsBuilder;
pub(crate) use crate::files::os_release::OsReleaseBuilder;
pub(crate) use crate::files::passwd::PasswdBuilder;
pub(crate) use crate::files::cpuinfo::CpuinfoBuilder;
pub(crate) use crate::files::crypto::CryptoBuilder;
pub(crate) use crate::files::filesystems::FilesystemBuilder;
pub(crate) use crate::files::loadavg::LoadAvgBuilder;
pub(crate) use crate::files::mdstat::MdstatBuilder;
pub(crate) use crate::files::meminfo::MeminfoBuilder;
pub(crate) use crate::files::mounts::MountsBuilder;
pub(crate) use crate::files::partitions::PartitionsBuilder;
pub(crate) use crate::files::swaps::SwapsBuilder;
pub(crate) use crate::files::uptime::UptimeBuilder;
pub(crate) use crate::files::version::VersionBuilder;

use std::fmt::{Display, Formatter};
use regex::Regex;
use serde::{Deserializer, Serialize};
use async_trait::async_trait;
use thiserror::Error;
use crate::system::os::Os;
use crate::system::System;
use crate::error::{Resul, Erro};
use crate::apps::Serializable;
use crate::description::{Description, DescriptionField};

/// Import all necessary dependencies for a file implementation with `use crate::file::prelude::*`
pub(crate) mod prelude {
    pub(crate) use crate::utils::{file_metadata, count};
    pub(crate) use super::{Capability, FileExample, FileMatchPattern, File, FileBuilder};
    pub(crate) use lazy_static::lazy_static;
    pub(crate) use serde::{Deserialize, Serialize, Deserializer};
    pub(crate) use async_trait::async_trait;
    pub(crate) use crate::error::*;
    pub(crate) use crate::system::System;
    pub(crate) use crate::system::os::*;
    pub(crate) use crate::description::*;
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) enum Capability {
    Read,
    Write,
    Delete,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Capability::Read => "read",
            Capability::Write => "write",
            Capability::Delete => "delete"
        })
    }
}

#[derive(Serialize)]
pub(crate) struct FileHelp<'a> {
    name: &'static str,
    description: &'static str,
    capabilities: &'static [Capability],
    patterns: &'a [FileMatchPattern],
    input: &'static DescriptionField,
    output: &'static DescriptionField,
    examples: &'a [FileExample],
}

#[derive(Serialize)]
pub(crate) struct ReadExample {
    description: &'static str,
    output: Serializable,
}

#[derive(Serialize)]
pub(crate) struct WriteExample {
    description: &'static str,
    input: Serializable,
}

/// Used for deletion but not common.
#[derive(Debug, Serialize, Clone)]
pub(crate) struct DeleteExample {
    description: &'static str,
}

/// An example struct for each case
#[derive(Serialize)]
pub(crate) enum FileExample {
    Get(ReadExample),
    Write(WriteExample),
    Delete(DeleteExample),
}

impl FileExample {
    /// Shorthand for get
    pub(crate) fn new_get<O: Serialize + Send + Sync + 'static>(description: &'static str, output: O) -> Self {
        FileExample::Get(ReadExample { output: Box::new(output), description })
    }

    /// Shorthand for write
    pub(crate) fn new_write<I: Serialize + Send + Sync + 'static>(description: &'static str, input: I) -> Self {
        FileExample::Write(WriteExample { input: Box::new(input), description })
    }

    /// Shorthand for delete
    pub(crate) fn new_delete() -> Self {
        FileExample::Delete(DeleteExample { description: "Delete the file" })
    }
}

/// `Path` for exact match and `Regex` for rest.
#[derive(Debug, Clone, Serialize)]
pub(crate) enum FileMatchPatternType {
    Path(String),
    #[serde(with = "serde_regex")]
    Regex(Regex),
}

/// To identify if a file implementation is applicable it must be identified in some way.
/// It works by matching the target operating systemd and the provided path.
#[derive(Clone, Serialize)]
pub(crate) struct FileMatchPattern {
    pattern: FileMatchPatternType,
    compatibility: Vec<Os>,
}

impl FileMatchPattern {
    /// Use regex only if necessary.
    pub(crate) fn new(pattern: FileMatchPatternType, compatibility: &[Os]) -> Self {
        Self {
            pattern,
            compatibility: compatibility.to_vec(),
        }
    }

    /// Shorthand for path
    pub(crate) fn new_path(path: &str, compatibility: &[Os]) -> Self {
        Self::new(FileMatchPatternType::Path(path.into()), compatibility)
    }

    /// Shorthand for regex
    pub(crate) fn new_regex(regex: Regex, compatibility: &[Os]) -> Self {
        Self::new(FileMatchPatternType::Regex(regex), compatibility)
    }

    /// This is called very often due to directory listing.
    pub(crate) fn r#match(&self, value: &str,
                          os: &Os) -> bool {
        if self.compatibility.iter().any(|i| i.compatible(os)) {
            match &self.pattern {
                FileMatchPatternType::Path(s) => s.as_str() == value,
                FileMatchPatternType::Regex(regex) => regex.is_match(value)
            }
        } else {
            false
        }
    }
}

#[async_trait]
pub(crate) trait File: Sync + Send {
    type Output: Serialize + Description;
    type Input: Description;

    fn new(path: &str) -> Self;

    async fn read(&self, _system: &System) -> Resul<Self::Output> {
        Err(FileError::NotCapable(Capability::Read)).map_err(Into::into)
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, _input: I, _system: &System) -> Resul<()> {
        Err(FileError::NotCapable(Capability::Write)).map_err(Into::into)
    }

    async fn delete(&self, system: &System) -> Resul<()> {
        system.delete(self.path()).await
    }

    fn path(&self) -> &str;

    fn input_description() -> &'static DescriptionField {
        Self::Input::field()
    }

    fn output_description() -> &'static DescriptionField {
        Self::Output::field()
    }
}

pub(crate) trait FileBuilder {
    type File: File;

    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const CAPABILITIES: &'static [Capability];

    /// List of patterns which matches on the target machine.
    /// The combination of operating system and path maybe different.
    fn patterns(&self) -> &[FileMatchPattern];

    /// Try to identify if this implementation can manage the file.
    fn r#match(&self, value: &str,
               os: &Os)
               -> Option<Self::File> {
        log::trace!("start matching {} with value {} and os {:?}", Self::NAME, value, os);

        for pattern in self.patterns() {
            if pattern.r#match(value, os) {
                return Some(Self::File::new(value));
            }
        }

        None
    }

    /// Useful examples for end user.
    fn examples(&self) -> &[FileExample] {
        &[]
    }

    /// Returns a documentation about all variables with their description.
    fn input(&self) -> &'static DescriptionField {
        Self::File::input_description()
    }

    /// Returns a documentation about all variables with their description.
    fn output(&self) -> &'static DescriptionField {
        Self::File::output_description()
    }

    /// Overview about all end user relevant information to interact with this implementation.
    fn help(&self) -> FileHelp {
        FileHelp {
            name: Self::NAME,
            description: Self::DESCRIPTION,
            capabilities: Self::CAPABILITIES,
            patterns: self.patterns(),
            input: self.input(),
            output: self.output(),
            examples: self.examples(),
        }
    }
}

macro_rules! file_builders {
    ($(
        $typ:tt
    ),*
    ) => {
        pub(crate) enum FileBuilders {
            $(
                $typ($typ),
            )*
        }

        impl FileBuilders {
           pub(crate) fn name(&self) -> &str {
                match self {
                    $( Self::$typ(_)  => $typ::NAME, )*
                }
            }

            pub(crate) fn r#match(&self, path: &str, os: &Os) -> bool {
                match self {
                    $( Self::$typ(i)  => i.r#match(path, os).is_some(), )*
                }
            }

           pub(crate) async fn read(&self, path: &str, system: &System) -> Resul<Box<dyn erased_serde::Serialize + Send>> {
                match self {
                    $( Self::$typ(i) => Ok(i.r#match(path, system.os()?).ok_or(Erro::FilesNotMatched)?.read(system).await.map(Box::new)?), )*
                }
            }

           #[allow(dead_code)]
            pub(crate) async fn read_bytes(&self, path: &str, system: &System) -> Resul<Vec<u8>> {
                match self {
                    $( Self::$typ(_i)  => system.read(path).await, )*
                }
            }

            pub(crate) async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, path: &str, input: I, system: &System) -> Resul<()> {
                match self {
                    $( Self::$typ(i)  => i.r#match(path, system.os()?).ok_or(Erro::FilesNotMatched)?.write(input, system).await, )*
                }
            }

           #[allow(dead_code)]
            pub(crate) async fn write_bytes(&self, path: &str, input: Vec<u8>, system: &System) -> Resul<()> {
                match self {
                    $( Self::$typ(_i)  => system.write(path, &input).await, )*
                }
            }

            pub(crate) async fn delete(&self, path: &str, system: &System) -> Resul<()> {
                match self {
                    $( Self::$typ(_i)  => system.delete(path).await, )*
                }
            }
            pub(crate) fn help(&self) -> FileHelp {
                match self {
                    $( Self::$typ(i)  => i.help(), )*
                }
            }
        }
    }
}

file_builders!(
    VersionBuilder,
    UptimeBuilder,
    SwapsBuilder,
    PartitionsBuilder,
    MountsBuilder,
    MeminfoBuilder,
    MdstatBuilder,
    LoadAvgBuilder,
    FilesystemBuilder,
    CryptoBuilder,
    CpuinfoBuilder,
    PasswdBuilder,
    OsReleaseBuilder,
    HostsBuilder,
    HostnameBuilder,
    FstabBuilder,
    CrontabBuilder,
    YamlBuilder,
    JsonBuilder,
    TextBuilder
);

#[derive(Debug, Error)]
pub(crate) enum FileError {
    #[error("{0} not capable")]
    NotCapable(Capability)
}