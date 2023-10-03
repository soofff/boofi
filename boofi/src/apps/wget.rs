use crate::apps::prelude::*;
use crate::system::System;

#[derive(Serialize, Deserialize, Description)]
pub(crate) struct WgetInput {
    output: Option::<String>,
    user: Option::<String>,
    password: Option::<String>,
    no_check_certificates: Option::<bool>,
    url: String,
}

impl From<WgetInput> for Vec<String> {
    fn from(value: WgetInput) -> Self {
        let mut arguments = vec![];

        if let Some(v) = value.user {
            arguments.push("--user".into());
            arguments.push(v)
        }
        if let Some(v) = value.password {
            arguments.push("--password".into());
            arguments.push(v)
        }
        if let Some(v) = value.output {
            arguments.push("-O".into());
            arguments.push(v)
        }
        if let Some(true) = value.no_check_certificates { arguments.push("--no-check-certificate".into()) }
        arguments.push(value.url);

        arguments
    }
}

pub(crate) struct Wget;

#[async_trait]
impl App for Wget {
    type Output = ();
    type Input = WgetInput;

    fn new() -> Self {
        Self {}
    }

    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, input: I, system: &System) -> Resul<Self::Output> {
        let i = WgetInput::deserialize(input).map_err(Erro::from_deserialize)?;

        let arguments: Vec<String> = i.into();

        system.run_args("/usr/bin/wget", arguments.as_slice()).await?;

        Ok(())
    }
}

#[derive(Clone)]
#[derive(Default)]
pub(crate) struct WgetBuilder {}


impl AppBuilder for WgetBuilder {
    type App = Wget;

    const NAME: &'static str = "wget";
    const DESCRIPTION: &'static str = "Wget with limited function.";
    const SUPPORTED_OS: &'static [Os] = &[Os::LinuxAny];


    fn examples(&self) -> &[AppExample] {
        lazy_static! {
            static ref EXAMPLE: [AppExample; 1] = [
                AppExample::new("Download a file to /tmp",
                                Box::new(WgetInput {
                                    output: Some("/tmp/index.html".to_string()),
                                    user: None,
                                    password: None,
                                    no_check_certificates: None,
                                    url: "https://google.de".to_string(),
                                }), Box::new(""))
                ];
            }

        EXAMPLE.as_slice()
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use crate::apps::App;
    use crate::apps::wget::{Wget};
    use crate::utils::test::system_user;

    #[tokio::test]
    async fn test_run() {
        let mut wget = Wget {};

        wget.run(json!({"url": "https://www.rust-lang.org/", "output": "/tmp/rustlang.html"}),
                 &system_user().await,
        ).await.unwrap();
    }
}