pub(crate) mod os;
pub(crate) mod posix;

use async_trait::async_trait;
use crate::error::{Erro, Resul};
use crate::system::os::Os;
use crate::system::posix::Posix;

#[derive(Debug, PartialEq)]
pub(crate) enum FileType {
    File,
    Directory,
    CharacterDevice,
    BlockDevice,
    NamedPipe,
    SymbolicLink,
    Socket,
}

impl FileType {
    #[allow(dead_code)]
    pub(crate) fn is_file(&self) -> bool {
        self == &Self::File
    }

    #[allow(dead_code)]
    pub(crate) fn is_directory(&self) -> bool {
        self == &Self::Directory
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Credential {
    username: String,
    password: String,
}

impl Credential {
    pub(crate) fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    pub(crate) fn username(&self) -> &str { self.username.as_str() }

    pub(crate) fn password(&self) -> &str { self.password.as_str() }
}

/// Defines necessary methods to perform platform specific actions.
#[async_trait]
pub(crate) trait PlatformActions {
    fn name() -> &'static str;

    /// Returns a new instance if it is responsible for the endpoint.
    async fn detect(credentials: Credential, endpoint: Option<&str>) -> Resul<Option<Self>> where Self: Sized;

    fn endpoint(&self) -> Option<&str>;

    fn credential(&self) -> &Credential;

    async fn verify_credential(&self) -> Resul<()>;

    /// call a program on local machine
    async fn run_user<T: AsRef<str> + Send + Sync>(&self, _path: &str, _arguments: &[T]) -> Resul<Vec<u8>> {
        Err(Erro::RunUserUnsupported(Self::name()))
    }

    /// call a program on remote machine
    async fn run_ssh<T: AsRef<str> + Send + Sync>(&self, _path: &str, _arguments: &[T]) -> Resul<Vec<u8>> {
        Err(Erro::RunUserUnsupported(Self::name()))
    }

    /// read a file on local machine
    async fn read_user(&self, _path: &str) -> Resul<Vec<u8>> {
        Err(Erro::ReadUserUnsupported(Self::name()))
    }

    /// read a file on remote machine
    async fn read_ssh(&self, _path: &str) -> Resul<Vec<u8>> {
        Err(Erro::ReadSshUnsupported(Self::name()))
    }

    /// write a file on local machine
    async fn write_user(&self, _path: &str, _content: &[u8]) -> Resul<()> {
        Err(Erro::WriteUserUnsupported(Self::name()))
    }

    /// write a file on remote machine
    async fn write_ssh(&self, _path: &str, _content: &[u8]) -> Resul<()> {
        Err(Erro::WriteSshUnsupported(Self::name()))
    }

    /// delete a file on local machine
    async fn delete_user(&self, _path: &str) -> Resul<()> {
        Err(Erro::DeleteUserUnsupported(Self::name()))
    }

    /// delete a file on remote machine
    async fn delete_ssh(&self, _path: &str) -> Resul<()> {
        Err(Erro::DeleteSshUnsupported(Self::name()))
    }

    /// run a program on remote or local with arguments
    async fn run_args<T: AsRef<str> + Send + Sync>(&self, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        if self.endpoint().is_some() {
            self.run_ssh(path, arguments).await
        } else {
            self.run_user(path, arguments).await
        }
    }

    /// run a program on local or remote
    async fn run(&self, path: &str) -> Resul<Vec<u8>> {
        self.run_args::<&str>(path, &[]).await
    }

    /// read a file on local or remote
    async fn read(&self, path: &str) -> Resul<Vec<u8>> {
        if self.endpoint().is_some() {
            self.read_ssh(path).await
        } else {
            self.read_user(path).await
        }
    }

    /// read a file on local or remote into string
    async fn read_to_string(&self, path: &str) -> Resul<String> {
        String::from_utf8(self.read(path).await?).map_err(Into::into)
    }

    /// write a file on remote or local
    async fn write(&self, path: &str, content: &[u8]) -> Resul<()> {
        if self.endpoint().is_some() {
            self.write_ssh(path, content).await
        } else {
            self.write_user(path, content).await
        }
    }

    /// delete a file on local or remote
    async fn delete(&self, path: &str) -> Resul<()> {
        if self.endpoint().is_some() {
            self.delete_ssh(path).await
        } else {
            self.delete_user(path).await
        }
    }

    /// detect the specific operating system release
    async fn detect_os(&self) -> Resul<Os>;

    /// returns the file type e.g. file, directory, ..
    async fn file_type(&self, _path: &str) -> Resul<FileType> {
        Err(Erro::FileTypeUnsupported)
    }

    /// returns if a file like type exist or not
    async fn exist(&self, _path: &str) -> Resul<bool> {
        Err(Erro::PathExistUnsupported)
    }
}

/// Available platforms
#[derive(Clone)]
pub(crate) enum Platform {
    Posix(Posix),
}

/// Interact between code and operating system
#[derive(Clone)]
pub(crate) struct System {
    platform: Platform,
    os: Option<Os>,
}

impl System {
    #[cfg(test)]
    pub(crate) fn new(platform: Platform, os: Option<Os>) -> Self {
        Self {
            platform,
            os,
        }
    }

    pub(crate) fn os(&self) -> Resul<&Os> {
        self.os.as_ref().ok_or(Erro::OsDetection)
    }

    pub(crate) async fn verify_credential(&self) -> Resul<()> {
        match &self.platform {
            Platform::Posix(posix) => posix.verify_credential().await
        }
    }

    async fn detect(credential: Credential, endpoint: Option<&str>) -> Resul<Self> {
        let platform = if let Some(t) = Posix::detect(credential.clone(), endpoint).await? {
            Platform::Posix(t)
        } else {
            return Err(Erro::EndpointIncompatible);
        };

        Ok(Self {
            platform,
            os: None,
        })
    }

    async fn detect_os(&mut self) -> Resul<&Os> {
        let os = match &self.platform {
            Platform::Posix(posix) => posix.detect_os().await
        }?;

        self.os = Some(os);
        self.os()
    }

    pub(crate) async fn run_args<T: AsRef<str> + Send + Sync>(&self, path: &str, arguments: &[T]) -> Resul<Vec<u8>> {
        match &self.platform {
            Platform::Posix(t) => {
                t.run_args(path, arguments).await
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn run(&self, path: &str) -> Resul<Vec<u8>> {
        match &self.platform {
            Platform::Posix(t) => {
                t.run(path).await
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn read(&self, path: &str) -> Resul<Vec<u8>> {
        match &self.platform {
            Platform::Posix(t) => {
                t.read(path).await
            }
        }
    }

    pub(crate) async fn read_to_string(&self, path: &str) -> Resul<String> {
        match &self.platform {
            Platform::Posix(t) => {
                t.read_to_string(path).await
            }
        }
    }

    pub(crate) async fn write(&self, path: &str, content: &[u8]) -> Resul<()> {
        match &self.platform {
            Platform::Posix(t) => {
                t.write(path, content).await
            }
        }
    }

    pub(crate) async fn delete(&self, path: &str) -> Resul<()> {
        match &self.platform {
            Platform::Posix(t) => {
                t.delete(path).await
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn file_type(&self, path: &str) -> Resul<FileType> {
        match &self.platform {
            Platform::Posix(t) => {
                t.file_type(path).await
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn path_exist(&self, path: &str) -> Resul<bool> {
        match &self.platform {
            Platform::Posix(t) => {
                t.exist(path).await
            }
        }
    }
}

/// Bring OS, endpoint and credentials together
pub(crate) struct SystemManager {
    system: Option<System>,
    endpoint: Option<String>,
}

impl SystemManager {
    pub(crate) fn new(endpoint: Option<&str>) -> Self {
        Self {
            system: None,
            endpoint: endpoint.map(ToString::to_string),
        }
    }

    pub(crate) async fn system_credential(&mut self, credential: Credential) -> Resul<&System> {
        self.system(credential).await
    }

    async fn system(&mut self, credential: Credential) -> Resul<&System> {
        if self.system.is_none() {
            let mut system = System::detect(credential, self.endpoint.as_deref()).await?;
            system.detect_os().await?; // initial os detection - stored to system
            self.system = Some(system);
        }

        self.system.as_ref().ok_or(Erro::SystemDetection)
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use crate::system::{SystemManager, Credential, FileType};
    use crate::utils::test::{PASSWORD, SSH_ENDPOINT, system_ssh, system_user, USERNAME};

    fn credential() -> Credential {
        Credential::new(USERNAME, PASSWORD)
    }

    fn endpoint() -> Option<&'static str> {
        Some(SSH_ENDPOINT)
    }

    #[tokio::test]
    async fn test_run() {
        let samples = [
            ("echo", ["test"].as_slice(), "test\n"),
            ("uname", ["-s"].as_slice(), "Linux\n"),
            ("sleep", ["1"].as_slice(), ""),
            ("true", [].as_slice(), ""),
            ("sh", ["-c", "echo test"].as_slice(), "test\n"),
            ("sh", ["-c", "ls /proc | grep uptime"].as_slice(), "uptime\n")
        ];

        for (command, args, expect) in samples {
            let mut system_manager = SystemManager::new(None);
            assert_eq!(system_manager.system(credential()).await.unwrap().run_args(command, args).await.unwrap(), expect.as_bytes());

            let mut system_manager = SystemManager::new(endpoint());
            assert_eq!(system_manager.system(credential()).await.unwrap().run_args(command, args).await.unwrap(), expect.as_bytes());
        }
    }

    #[tokio::test]
    async fn test_run_failure() {
        let mut system_manager = SystemManager::new(None);
        assert!(format!("{:?}", &system_manager.system(credential()).await.unwrap().run("true1").await).contains(r#"not found"#));

        let mut system_manager = SystemManager::new(endpoint());
        assert!(format!("{:?}", &system_manager.system(credential()).await.unwrap().run("true1").await).contains(r#"not found"#));
    }

    #[tokio::test]
    async fn test_read_write_delete() {
        let path = "/tmp/testwritefile";
        let content = "text\nenter\n\n";

        // USER
        let mut system_manager = SystemManager::new(None);
        let system = system_manager.system(credential()).await.unwrap();
        system.write(path, content.as_bytes()).await.unwrap();

        let s = system.read_to_string(path).await.unwrap();
        assert_eq!(content, s.as_str());

        system.delete(path).await.unwrap();
        assert!(!Path::new(path).exists());

        // SSH
        let mut system_manager = SystemManager::new(endpoint());
        let system = system_manager.system(credential()).await.unwrap();
        system.write(path, content.as_bytes()).await.unwrap();

        let s = system.read_to_string(path).await.unwrap();
        assert_eq!(content, s.as_str());

        system.delete(path).await.unwrap();
        assert!(!Path::new(path).exists());
    }


    #[tokio::test]
    async fn test_run_file_type() {
        for (file, expect) in [
            /* todo failed in docker environment due to missing type
            ("/run/initctl", FileType::NamedPipe),
            ("/run/acpid.socket", FileType::Socket),
            ("/dev/sr0", FileType::BlockDevice),
            */
            ("/etc", FileType::Directory),
            ("/dev/null", FileType::CharacterDevice),
            ("/etc/hosts", FileType::File),
            ("/proc/self", FileType::SymbolicLink),
        ] {
            let system = system_user().await;
            assert_eq!(system.file_type(file).await.unwrap(), expect);
        }
    }

    #[tokio::test]
    async fn test_path_exist() {
        let exist = "/etc/fstab";
        let not = "/e/t/c/f/s/t/a/b";

        let system = system_user().await;
        assert!(system.path_exist(exist).await.unwrap());
        assert!(!system.path_exist(not).await.unwrap());

        let system = system_ssh().await;
        assert!(system.path_exist(exist).await.unwrap());
        assert!(!system.path_exist(not).await.unwrap());
    }
}