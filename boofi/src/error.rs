use std::convert::Infallible;
use semver::Error as SemverError;
use std::io::Error as IoError;
use std::net::AddrParseError;
use regex::Error as RegexError;
use std::num::{ParseFloatError, ParseIntError};
use std::string::FromUtf8Error;
use axum::extract::rejection::JsonRejection;
use axum::http::header::{InvalidHeaderValue, ToStrError};
use base64::DecodeError;
use serde_json::Error as SerdeJsonError;
use ssh_rs::error::SshError;
use axum::http::{Error as AxumError, Method};
use hyper::Error as HyperError;
use async_ssh2_tokio::Error as AsyncSshError;
use rcgen::RcgenError;
use thiserror::Error;
use tokio::task::JoinError;
use crate::files::hosts::HostsError;
use crate::files::passwd::PasswdError;
use crate::apps::uname::UnameError;
use crate::files::crontab::CrontabError;
use crate::files::crypto::CryptoError;
use crate::files::FileError;
use crate::files::loadavg::LoadAvgError;
use crate::files::mdstat::MdstatError;
use crate::files::version::VersionError;
use crate::files::os_release::OsReleaseError;

/// Manages and converts all errors
/// File/app implementations have their own error type which needs conversion
#[derive(Debug, Error)]
#[error("{0}")]
pub(crate) enum Erro {
    #[error("host detection failed")]
    SystemDetection,
    #[error("os detection failed")]
    OsDetection,
    #[error("no compatible platform found")]
    EndpointIncompatible,
    #[error("run user not supported for {0}")]
    RunUserUnsupported(&'static str),
    #[error("read user not supported for {0}")]
    ReadUserUnsupported(&'static str),
    #[error("read ssh not supported for {0}")]
    ReadSshUnsupported(&'static str),
    #[error("write user not supported for {0}")]
    WriteUserUnsupported(&'static str),
    #[error("write ssh not supported for {0}")]
    WriteSshUnsupported(&'static str),
    #[error("delete user not supported for {0}")]
    DeleteUserUnsupported(&'static str),
    #[error("delete ssh not supported for {0}")]
    DeleteSshUnsupported(&'static str),
    #[error("run user but user is invalid")]
    RunUserUserInvalid,
    #[error("run user but password is invalid")]
    RunUserPasswordInvalid,
    #[error("run user but issues with password stdin")]
    RunUserStdin,
    #[error("run user with exit code {0} and message: {1}")]
    RunUser(u32, String),
    #[error("run ssh with exit code {0} and message: {1}")]
    RunSsh(u32, String),
    #[error("endpoint missing")]
    EndpointMissing,
    #[error("write user but temporary file path is invalid")]
    WriteUserTempPath,
    #[error("operating system detection failed")]
    OsDetectionFailed,
    #[error("authentication missing")]
    RestAuthMissing,
    #[error("unsupported authentication method")]
    RestAuthInvalid,
    #[error("app is incompatible")]
    AppIncompatible,
    #[error("app not found")]
    AppNotFound,
    #[error("body missing")]
    AppBodyMissing,
    #[error("method {0} not allowed")]
    HttpMethodNotAllowed(Method),
    #[error("task not found")]
    TaskNotFound,
    #[error("file size unknown")]
    DirFileSizeUnknown,
    #[error("task index invalid")]
    TaskInvalidIndex,
    #[error("path invalid")]
    PathInvalid,
    #[error("File type unsupported")]
    FileTypeUnsupported,
    #[error("path exist unsupported")]
    PathExistUnsupported,
    #[error("File type {0} unknown")]
    FileTypeUnknown(String),
    #[error("nothing matched")]
    FilesNotMatched,
    #[error("nothing matched by name {0}")]
    FilesNotMatchedByName(String),
    #[error("nothing matched by pattern {0}")]
    FilesNotMatchedByPattern(String),
    #[error("failed to execute child process")]
    AuthTokenExpired,
    #[error("no authentication found")]
    AuthNotFound,
    #[error("private key path")]
    PrivateKeyPath,
    #[error("certificate path")]
    CertificatePath,
    Deserialize(String),

    // file/app errors
    File(#[from] FileError),
    Hosts(#[from] HostsError),
    Mdstat(#[from] MdstatError),
    Crypto(#[from] CryptoError),
    LoadAvg(#[from] LoadAvgError),
    Version(#[from] VersionError),
    Cron(#[from] CrontabError),
    Uname(#[from] UnameError),
    Passwd(#[from] PasswdError),
    OsRelease(#[from] OsReleaseError),

    // extern crate errors
    Semver(#[from] SemverError),
    Io(#[from] IoError),
    Regex(#[from] RegexError),
    ParseInt(#[from] ParseIntError),
    SerdeJson(#[from] SerdeJsonError),
    FromUtf8(#[from] FromUtf8Error),
    Ssh(#[from] SshError),
    ParseFloat(#[from] ParseFloatError),
    JsonRejection(#[from] JsonRejection),
    ToStrError(#[from] ToStrError),
    Base64Decode(#[from] DecodeError),
    Http(#[from] AxumError),
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    HyperError(#[from] HyperError),
    AsyncSsh(#[from] AsyncSshError),
    Yaml(#[from] serde_yaml::Error),
    AddrParse(#[from] AddrParseError),
    Join(#[from] JoinError),
    Rcgen(#[from] RcgenError),
    Rustls(#[from] rustls::Error),
    Infallible(#[from] Infallible),
}

/// Common result type
pub(crate) type Resul<T, E = Erro> = Result<T, E>;

impl Erro {
    // conversion workaround
    pub(crate) fn from_deserialize<T: serde::de::Error>(error: T) -> Self {
        Self::Deserialize(error.to_string())
    }
}