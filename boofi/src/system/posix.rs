use std::net::{TcpStream};
use std::process::{Stdio};
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
use async_trait::async_trait;
use ssh_rs::{SessionBuilder, SessionConnector};

use tokio::spawn;
use crate::apps::prelude::Os;
use crate::error::{Erro, Resul};

use crate::files::version::Version;
use crate::system::{PlatformActions, Credential, FileType};
use std::io::Write;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use crate::files::os_release::OsRelease;

/// Compatible with most linux distributions
#[derive(Clone)]
pub(crate) struct Posix {
    credential: Credential,
    endpoint: Option<String>,
}

impl Posix {
    #[cfg(test)]
    pub(crate) fn new(credential: Credential, endpoint: Option<String>) -> Self {
        Self {
            credential,
            endpoint,
        }
    }

    fn su() -> &'static str {
        "/bin/su"
    }

    fn unlink() -> &'static str {
        "/bin/unlink"
    }

    fn stat() -> &'static str {
        "/bin/stat"
    }

    fn r#true() -> &'static str {
        "/bin/true"
    }

    fn cp() -> &'static str {
        "/bin/cp"
    }

    fn cat() -> &'static str {
        "/bin/cat"
    }

    fn chmod() -> &'static str {
        "/bin/chmod"
    }

    fn test() -> &'static str { "/bin/test" }

    /// call a program as user with provided password using `su`
    async fn run_user<T: AsRef<str>>(username: &str, password: &str, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        let mut args = vec![path];

        for arg in arguments {
            args.push(arg.as_ref())
        }

        let mut command = Command::new(Self::su());
        command.args([
            username,
            "-c",
            &args.iter().map(|s| format!(r#""{}""#, s)).collect::<Vec<String>>().join(" ")
        ]);

        log::debug!("[RUN USER] execute {} {} -c {:?}", Self::su(), username, args);

        let mut child = command.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().ok_or(Erro::RunUserStdin)?;

        let pw = password.to_string();

        spawn(async move {
            log::trace!("[RUN USER] pass password to stdin");
            if let Err(e) = stdin.write_all(pw.as_bytes()).await {
                log::error!("[RUN USER] {}", e);
            }
        });

        let output = child.wait_with_output().await?;

        let result = if output.status.success() {
            output.stdout
        } else {
            let err = String::from_utf8(output.stderr)?;
            let code = output.status.code().unwrap_or(1) as u32;

            log::error!("[RUN USER] execution failed with code {} and output {}", code, err);

            // catch credential errors and su prefixes
            if err.trim().to_lowercase().contains("password: su: authentication failure") {
                return Err(Erro::RunUserPasswordInvalid).map_err(Into::into);
            }

            if err.starts_with("su: user") && err.contains("does not exist") {
                return Err(Erro::RunUserUserInvalid).map_err(Into::into);
            }

            return Err(Erro::RunUser(code,
                                     if err.to_lowercase().starts_with("password: ") {
                                         err[10..].into()
                                     } else {
                                         err
                                     },
            ));
        };

        log::debug!("[RUN USER] finished");

        Ok(result)
    }

    /// use ssh2 to connect to the endpoint.
    /// current implementation does not allow raw byte stream (u8 is just dirty string conversion)
    async fn run_ssh<T: AsRef<str>>(client: Client, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        let mut args = vec![path.to_string()];

        for arg in arguments {
            args.push(format!(r#""{}""#, arg.as_ref()));
        }

        let command = args.join(" ");

        log::debug!("[RUN SSH] execute {}", command);

        let result = client.execute(&command).await?;

        if result.exit_status > 0 {
            log::error!("[RUN SSH] exit code {} and output: {}", result.exit_status, result.stderr);
            return Err(Erro::RunSsh(result.exit_status, result.stderr));
        }

        log::trace!("[RUN SSH] finished with output {}", result.stdout);

        // todo: use byte stream somehow ? (russh)
        Ok(result.stdout.into_bytes())
    }

    async fn ssh_connect(endpoint: &str, username: &str, password: &str) -> Resul<Client> {
        log::debug!("[SSH CONNECT] connecting to {:?}", endpoint);
        Client::connect(
            endpoint,
            username,
            AuthMethod::with_password(password),
            ServerCheckMethod::NoCheck,
        ).await.map_err(Into::into)
    }

    fn ssh_connect_scp(&self) -> Resul<SessionConnector<TcpStream>> {
        log::debug!("[SSH SCP] connecting to {:?}", self.endpoint);

        let credential = self.credential();

        SessionBuilder::new()
            .username(credential.username())
            .password(credential.password())
            .connect(self.endpoint_ok()?)
            .map_err(Into::into)
    }

    /// option to result
    fn endpoint_ok(&self) -> Resul<&str> {
        self.endpoint.as_deref().ok_or(Erro::EndpointMissing)
    }
}

#[async_trait]
impl PlatformActions for Posix {
    fn name() -> &'static str {
        "posix"
    }

    async fn detect(credential: Credential, endpoint: Option<&str>) -> Resul<Option<Self>> {
        let executables = &[
            Self::su(),
            Self::unlink(),
            Self::r#true(),
            Self::cp(),
            Self::cat(),
            Self::chmod(),
            Self::test(),
        ];

        if let Some(e) = endpoint {
            let client = Self::ssh_connect(e, credential.username(), credential.password()).await?;
            Self::run_ssh(client, Self::stat(), executables).await?;
        } else {
            Self::run_user(credential.username(), credential.password(), Self::stat(), executables).await?;
        }

        log::info!("{} compatibility check successful", Self::name());
        Ok(Some(Self {
            credential,
            endpoint: endpoint.map(ToString::to_string),
        }))
    }

    fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    fn credential(&self) -> &Credential {
        &self.credential
    }

    async fn verify_credential(&self) -> Resul<()> {
        self.run(Self::r#true()).await.map(|_| ())
    }

    async fn run_user<T: AsRef<str> + Send + Sync>(&self, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        Self::run_user(self.credential().username(), self.credential().password(), path, arguments).await
    }

    async fn run_ssh<T: AsRef<str> + Send + Sync>(&self, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        let client = Self::ssh_connect(self.endpoint_ok()?, self.credential().username(), self.credential().password()).await?;
        Self::run_ssh(client, path, arguments).await
    }

    async fn read_user(&self, path: &str) -> Resul<Vec<u8>> {
        self.run_user(Self::cat(), &[path]).await
    }

    async fn read_ssh(&self, path: &str) -> Resul<Vec<u8>> {
        log::debug!("[READ SSH] reading {}", path);
        self.run_args(Self::cat(), &[path]).await
    }

    /// use temporary file, `cp` and `chmod` to create/write file
    async fn write_user(&self, path: &str, content: &[u8]) -> Resul<()> {
        let mut temp = tempfile::NamedTempFile::new()?;

        log::debug!("[WRITE USER] writing bytes to {:?}", temp.path());
        temp.write_all(content)?;

        let tmp_path_str = temp.path().to_str().ok_or(Erro::WriteUserTempPath)?;

        Command::new(Self::chmod()).args(["444", tmp_path_str]).output().await?;

        log::debug!("[WRITE USER] copy from {:?} to {:?}", temp.path(), path);
        self.run_user(Self::cp(), &[
            "--no-preserve=mode,ownership", // ignore chmod workaround
            tmp_path_str,
            path
        ]).await?;

        temp.close().map_err(Into::into)
    }

    /// use temporary file and scp to write to file
    async fn write_ssh(&self, path: &str, content: &[u8]) -> Resul<()> {
        log::trace!("[WRITE SSH] connecting ssh scp");
        let exec = self.ssh_connect_scp()?.run_local().open_scp()?;
        let mut temp = tempfile::NamedTempFile::new()?;
        log::debug!("[WRITE SSH] writing bytes to {:?}", temp.path());
        temp.write_all(content)?;
        log::debug!("[WRITE SSH] upload local {:?} to remote {:?}", temp.path(), path);
        exec.upload(temp.path(), path.as_ref())?;
        temp.close().map_err(Into::into)
    }

    async fn delete_user(&self, path: &str) -> Resul<()> {
        self.run_user(Self::unlink(), &[path]).await.map(|_| {})
    }

    async fn delete_ssh(&self, path: &str) -> Resul<()> {
        self.run_ssh(Self::unlink(), &[path]).await.map(|_| {})
    }

    async fn detect_os(&self) -> Resul<Os> {
        if Version::parse(&self.read_to_string("/proc/version").await?)?.version().contains("Linux") {
            log::debug!("[DETECT] Linux detected");

            let os: Os = if let Ok(s) = self.read_to_string("/etc/os-release").await {
                let release = OsRelease::try_from(s)?;

                match release.id() {
                    "ubuntu" | "debian" => release.version_codename().unwrap_or(release.id()).parse()?,
                    _ => release.id().parse()?
                }
            } else {
                Os::LinuxUnknown
            };

            log::debug!("[DETECT] {:?} detected", os);

            Ok(os)
        } else {
            Err(Erro::OsDetectionFailed)
        }
    }

    async fn file_type(&self, path: &str) -> Resul<FileType> {
        Ok(match String::from_utf8(self.run_args(Self::stat(), &["--printf", "%F", path]).await?)?.as_str() {
            "socket" => FileType::Socket,
            "directory" => FileType::Directory,
            "regular file" | "regular empty file" => FileType::File,
            "block special file" => FileType::BlockDevice,
            "symbolic link" => FileType::SymbolicLink,
            "character special file" => FileType::CharacterDevice,
            "fifo" => FileType::NamedPipe,
            _ => return Err(Erro::FileTypeUnknown(path.to_string()))
        })
    }

    async fn exist(&self, path: &str) -> Resul<bool> {
        let result = self.run_args(Self::test(), &["-e", path]).await;

        match result {
            Ok(_) => Ok(true),
            Err(Erro::RunUser(code, _)) |
            Err(Erro::RunSsh(code, _)) if code == 1 => Ok(false),
            Err(e) => Err(e)
        }
    }
}
