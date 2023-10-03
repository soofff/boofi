use std::time::{Duration, SystemTime};
use rand::Rng;
use crate::apps::*;
use crate::files::*;
use crate::error::{Erro, Resul};
use crate::system::{System, SystemManager};
use crate::task::TaskController;

/// Stores authentication data
pub(crate) struct Auth {
    token: String,
    username: String,
    password: String,
    date: SystemTime,
}

impl Auth {
    fn expired(&self, duration: Duration) -> bool {
        SystemTime::now() >= self.date + duration
    }

    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }
}

/// Manages all credentials and checks expiration.
pub(crate) struct AuthController {
    auths: Vec<Auth>,
    duration: Duration,
}

impl AuthController {
    fn token() -> String {
        rand::thread_rng().sample_iter(rand::distributions::Alphanumeric).take(16).map(char::from).collect()
    }

    /// Add or update a new token
    pub(crate) fn insert_or_replace(&mut self, username: String, password: String) -> String {
        for auth in self.auths.iter_mut() {
            if auth.username == username {
                auth.password = password;
                auth.token = Self::token();
                return auth.token.clone();
            }
        }

        let token = Self::token();
        self.auths.push(Auth {
            token: token.clone(),
            username,
            password,
            date: SystemTime::now(),
        });

        token
    }

    pub(crate) fn get(&self, token: &str) -> Resul<&Auth> {
        self.auths.iter().find(|auth| {
            auth.token == token
        }).map(|auth| {
            if auth.expired(self.duration) {
                Err(Erro::AuthTokenExpired)
            } else {
                Ok(auth)
            }
        }).ok_or(Erro::AuthNotFound)?
    }

    pub(crate) fn delete(&mut self, token: &str) -> bool {
        let i = self.auths.len();
        self.auths.retain(|auth| auth.token != token);
        i > self.auths.len()
    }
}

/// Manages all apps/files/tasks + authentication
/// Used for one target/endpoint
pub(crate) struct Controller {
    files: Vec<FileBuilders>,
    apps: Vec<AppBuilders>,
    task_controller: TaskController,
    auth: AuthController,
    system_manager: SystemManager,
}

impl Controller {
    /// Instantiate a new controller for local or ssh endpoint
    pub(crate) async fn new(max_token_expiration: Duration, address: Option<&str>) -> Resul<Self> {
        let system_manager = SystemManager::new(address);

        log::debug!("loading file builders");
        let mut files = vec![];

        for file in [
            FileBuilders::VersionBuilder(VersionBuilder {}),
            FileBuilders::UptimeBuilder(UptimeBuilder {}),
            FileBuilders::SwapsBuilder(SwapsBuilder {}),
            FileBuilders::PartitionsBuilder(PartitionsBuilder {}),
            FileBuilders::MountsBuilder(MountsBuilder {}),
            FileBuilders::MeminfoBuilder(MeminfoBuilder {}),
            FileBuilders::MdstatBuilder(MdstatBuilder {}),
            FileBuilders::LoadAvgBuilder(LoadAvgBuilder {}),
            FileBuilders::FilesystemBuilder(FilesystemBuilder {}),
            FileBuilders::CryptoBuilder(CryptoBuilder {}),
            FileBuilders::CpuinfoBuilder(CpuinfoBuilder {}),
            FileBuilders::PasswdBuilder(PasswdBuilder {}),
            FileBuilders::OsReleaseBuilder(OsReleaseBuilder {}),
            FileBuilders::HostsBuilder(HostsBuilder {}),
            FileBuilders::HostnameBuilder(HostnameBuilder {}),
            FileBuilders::FstabBuilder(FstabBuilder {}),
            FileBuilders::CrontabBuilder(CrontabBuilder {}),
            FileBuilders::YamlBuilder(YamlBuilder {}),
            FileBuilders::JsonBuilder(JsonBuilder {}),
            FileBuilders::TextBuilder(TextBuilder {}),
        ].into_iter() {
            files.push(file);
            log::info!("file builder '{}' loaded", files[files.len()-1].name());
        }

        log::debug!("loading app builders");
        let mut apps = vec![];
        for app in [
            AppBuilders::LsBuilder(LsBuilder::default()),
            AppBuilders::UnameBuilder(UnameBuilder::default()),
            AppBuilders::WgetBuilder(WgetBuilder::default()),
            AppBuilders::TouchBuilder(TouchBuilder::default()),
            AppBuilders::ShBuilder(ShBuilder::default()),
        ].into_iter() {
            apps.push(app);
            log::info!("app builder '{}' loaded", apps[apps.len()-1].name());
        }

        Ok(Self {
            files,
            apps,
            task_controller: TaskController::default(),
            auth: AuthController {
                auths: vec![],
                duration: max_token_expiration,
            },
            system_manager,
        })
    }

    pub(crate) fn system_manager_mut(&mut self) -> &mut SystemManager {
        &mut self.system_manager
    }

    pub(crate) fn auth_mut(&mut self) -> &mut AuthController {
        &mut self.auth
    }

    pub(crate) fn file_builders_mut(&mut self, name: &str) -> Resul<&mut FileBuilders> {
        log::debug!("[FILE] trying to get by name {}",name);

        for f in self.files.iter_mut() {
            log::trace!("[FILE] trying name {}",name);

            if f.name() == name {
                log::debug!("[FILE] {} found",name);
                return Ok(f);
            }
        }
        log::debug!("[FILE] nothing found by name {}",name);
        Err(Erro::FilesNotMatchedByName(name.into()))
    }

    pub(crate) async fn file_builders_mut_by_match(&mut self, pattern: &str, system: &System) -> Resul<&mut FileBuilders> {
        log::debug!("[FILE MATCH] trying to match file by pattern {}", pattern);
        let os = system.os()?;
        self.files.iter_mut().find(|f| f.r#match(pattern, os))
            .ok_or(Erro::FilesNotMatchedByPattern(pattern.into()))
    }

    pub(crate) fn file_builders(&self) -> &[FileBuilders] {
        self.files.as_slice()
    }

    pub(crate) fn apps(&self) -> &[AppBuilders] {
        &self.apps
    }

    pub(crate) fn app(&self, name: &str) -> Option<&AppBuilders> {
        self.apps.iter().find(|app| app.name() == name)
    }

    pub(crate) fn app_mut(&mut self, name: &str) -> Option<&mut AppBuilders> {
        self.apps.iter_mut().find(|app| app.name() == name)
    }

    pub(crate) fn task_controller(&self) -> &TaskController {
        &self.task_controller
    }

    pub(crate) fn task_controller_mut(&mut self) -> &mut TaskController {
        &mut self.task_controller
    }
}

#[cfg(test)]
mod tests {
    use crate::controller::AuthController;

    #[test]
    fn token_expired() {
        let mut auth = AuthController {
            auths: vec![],
            duration: Default::default(),
        };

        let token = auth.insert_or_replace("user".into(), "pass".into());
        assert!(auth.get(&token).is_err());
    }

    #[test]
    fn token_remove() {
        let mut auth = AuthController {
            auths: vec![],
            duration: Default::default(),
        };

        let token = auth.insert_or_replace("user".into(), "pass".into());

        assert!(auth.delete(&token));
        assert!(!auth.delete(&token));
    }
}
