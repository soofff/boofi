use std::vec;
use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Description)]
pub(crate) struct PasswdEntry {
    user: String,
    password: String,
    user_id: usize,
    group_id: usize,
    comment: String,
    home: String,
    program: String,
}

impl ToString for PasswdEntry {
    fn to_string(&self) -> String {
        format!("{}:{}:{}:{}:{}:{}:{}",
                self.user,
                self.password,
                self.user_id,
                self.group_id,
                self.comment,
                self.home,
                self.program,
        )
    }
}


impl TryFrom<String> for PasswdEntry {
    type Error = Erro;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parts: Vec<String> = value.split(':').map(ToString::to_string).collect();
        Ok(Self {
            user: parts.remove(0),
            password: parts.remove(0),
            user_id: parts.remove(0).parse()?,
            group_id: parts.remove(0).parse()?,
            comment: parts.remove(0),
            home: parts.remove(0),
            program: parts.remove(0),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Description)]
pub(crate) struct Passwd {
    content: Vec<PasswdEntry>,
}

impl Passwd {
    fn parse(content: &str) -> Resul<Self> {
        content.split('\n')
            .filter_map(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(PasswdEntry::try_from(s.to_string()))
                }
            })
            .collect::<Resul<Vec<PasswdEntry>>>()
            .map(|entries| {
                Self {
                    content: entries
                }
            })
    }


    fn content(&self) -> &[PasswdEntry] {
        self.content.as_slice()
    }

    fn content_string(&self) -> String {
        let s: Vec<String> = self.content
            .iter()
            .map(ToString::to_string)
            .collect();
        let mut r = s.join("\n");
        r.push('\n');
        r
    }

    fn add_user(&mut self, entry: PasswdEntry) -> Result<(), PasswdError> {
        if !self.content
            .iter().any(|e| e.user == entry.user) {
            self.content.push(entry);
            Ok(())
        } else {
            Err(PasswdError::UserAlreadyExist(entry.user))
        }
    }

    fn remove_user(&mut self, username: &str) -> Result<(), PasswdError> {
        let len = self.content.len();
        self.content.retain(|entry| entry.user != username);

        if len == self.content().len() {
            Err(PasswdError::UserNotFound(username.into()))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PasswdBuilder;

#[async_trait]
impl File for PasswdFile {
    type Output = Passwd;
    type Input = PasswdInput;

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Passwd::parse(&system.read_to_string(&self.path).await?)
    }

    async fn write<'de, I: Deserializer<'de> + Send + Sync>(&self, input: I, system: &System) -> Resul<()> {
        let i = PasswdInput::deserialize(input).map_err(Erro::from_deserialize)?;

        if i.overwrite == Some(true) {
            if let Some(new_entries) = i.new_entries {
                system.write(&self.path, Passwd {
                    content: new_entries
                }.content_string().as_bytes()).await
            } else {
                Err(PasswdError::NoNewEntries.into())
            }
        } else {
            let mut passwd = Passwd::parse(&system.read_to_string(self.path()).await?)?;

            if let Some(new) = i.new_entries {
                for e in new.into_iter() {
                    passwd.add_user(e)?;
                }
            }

            if let Some(usernames) = i.remove_by_username {
                for username in usernames.into_iter() {
                    passwd.remove_user(&username)?;
                }
            }

            system.write(self.path(), passwd.content_string().as_bytes()).await
        }
    }
    fn path(&self) -> &str {
        &self.path
    }
}

impl FileBuilder for PasswdBuilder {
    type File = PasswdFile;

    const NAME: &'static str = "passwd";
    const DESCRIPTION: &'static str = "Managed passwd file.";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read, Capability::Write, Capability::Delete];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref PATTERN: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/etc/passwd", &[Os::LinuxAny])];
        }
        PATTERN.as_slice()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLES: Vec<FileExample> = vec![
                FileExample::new_get("Example content", vec![PasswdEntry {
                    user: "root".to_string(),
                    password: "x".to_string(),
                    user_id: 0,
                    group_id: 0,
                    comment: "super user".to_string(),
                    home: "/root".to_string(),
                    program: "/bin/bash".to_string(),
                }]),
                FileExample::new_write("Add an user and remove another one.", PasswdInput {
                    new_entries: Some(vec![PasswdEntry {
                        user: "homer".to_string(),
                        password: "x".to_string(),
                        user_id: 1000,
                        group_id: 1000,
                        comment: "wohoo".to_string(),
                        home: "/home/homer".to_string(),
                        program: "/bin/sh".to_string(),
                    }]),
                    remove_by_username: Some(vec!["bart".to_string()]),
                    overwrite: Some(false)
                }),
                FileExample::new_delete(),
            ];
        }

        EXAMPLES.as_slice()
    }
}

#[derive(Debug)]
pub(crate) struct PasswdFile {
    path: String,
}

#[derive(Serialize, Deserialize, Description)]
pub(crate) struct PasswdInput {
    new_entries: Option<Vec<PasswdEntry>>,
    remove_by_username: Option<Vec<String>>,
    overwrite: Option<bool>,
}


#[derive(Debug, Error)]
pub(crate) enum PasswdError {
    #[error("user {0} already exist")]
    UserAlreadyExist(String),
    #[error("user {0} not found")]
    UserNotFound(String),
    #[error("no new entries was given")]
    NoNewEntries,
}

#[cfg(test)]
mod test {
    use crate::files::passwd::{Passwd, PasswdEntry};
    use crate::utils::test::read_test_resources;

    #[test]
    fn test_parse() {
        let content = read_test_resources("passwd");
        let passwd = Passwd::parse(&content).unwrap();

        assert_eq!(passwd.content, vec![
            PasswdEntry { user: "root".into(), password: "x".into(), user_id: 0, group_id: 0, comment: "root".into(), home: "/root".into(), program: "/bin/bash".into() },
            PasswdEntry { user: "bin".into(), password: "x".into(), user_id: 2, group_id: 2, comment: "bin".into(), home: "/bin".into(), program: "/usr/sbin/nologin".into() },
            PasswdEntry { user: "dev".into(), password: "x".into(), user_id: 1001, group_id: 1001, comment: "".into(), home: "/home/dev".into(), program: "/bin/sh".into() },
        ]);

        assert_eq!(passwd.content_string(), content);
    }

    #[test]
    fn test_add() {
        let mut passwd = Passwd {
            content: vec![],
        };

        let entry = PasswdEntry {
            user: "test".to_string(),
            password: "x".to_string(),
            user_id: 1,
            group_id: 2,
            comment: "".to_string(),
            home: "".to_string(),
            program: "".to_string(),
        };

        passwd.add_user(entry.clone()).unwrap();

        assert_eq!(passwd.content, vec![entry.clone()]);

        let mut entry2 = entry.clone();
        entry2.user = "test2".into();

        passwd.add_user(entry2.clone()).unwrap();

        // add another one
        assert_eq!(passwd.content, vec![entry.clone(), entry2]);

        // duplicate
        assert_eq!(&format!("{:?}", passwd.add_user(entry)), "Err(UserAlreadyExist(\"test\"))");
    }

    #[test]
    fn test_remove() {
        let user1 = PasswdEntry {
            user: "test".to_string(),
            password: "x".to_string(),
            user_id: 1,
            group_id: 2,
            comment: "".to_string(),
            home: "".to_string(),
            program: "".to_string(),
        };

        let user2 = PasswdEntry {
            user: "test2".to_string(),
            password: "x".to_string(),
            user_id: 2,
            group_id: 3,
            comment: "".to_string(),
            home: "".to_string(),
            program: "".to_string(),
        };

        let mut passwd = Passwd {
            content: vec![
                user1, user2.clone(),
            ],
        };

        passwd.remove_user("test").unwrap();

        assert_eq!(passwd, Passwd {
            content: vec![user2]
        });

        // already gone
        assert_eq!(&format!("{:?}", passwd.remove_user("test")), "Err(UserNotFound(\"test\"))");
    }
}