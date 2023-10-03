use crate::apps::prelude::*;
use crate::system::System;

#[derive(Serialize, Deserialize, Description)]
pub(crate) struct ShInput {
    command: String,
}

impl From<ShInput> for Vec<String> {
    fn from(value: ShInput) -> Self {
        vec!["-c".into(), value.command]
    }
}

pub(crate) struct Sh {}

#[async_trait]
impl App for Sh {
    type Output = String;
    type Input = ShInput;

    fn new() -> Self {
        Self {}
    }

    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, input: I, system: &System) -> Resul<Self::Output> {
        let input = ShInput::deserialize(input).map_err(Erro::from_deserialize)?;
        let args: Vec<String> = input.into();

        system.run_args("/bin/sh",
                        args.as_slice(),
        ).await.map(String::from_utf8)?.map_err(Into::into)
    }
}

#[derive(Clone)]
#[derive(Default)]
pub(crate) struct ShBuilder;

impl AppBuilder for ShBuilder {
    app_metadata!(
        Sh,
        "sh",
        "Shell",
        &[Os::LinuxAny],
        AppExample::new("Run command",
            Box::new(ShInput {
                command: "whoami".into()
            }),
            Box::new("root\n")
        )
    );
}


#[cfg(test)]
mod test {
    use serde_json::to_value;
    use crate::apps::App;
    use crate::apps::sh::{Sh, ShInput};
    use crate::utils::test::system_user;

    #[tokio::test]
    async fn test_run() {
        let mut sh = Sh {};

        let result = sh.run(to_value(ShInput {
            command: "echo test".into(),
        }).unwrap(), &system_user().await).await.unwrap();

        assert_eq!(result, "test\n");
    }
}