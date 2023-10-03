use crate::apps::prelude::*;
use crate::system::System;

#[derive(Serialize, Deserialize, Description)]
pub(crate) struct TouchInput {
    path: String,
}

pub(crate) struct Touch;

#[async_trait]
impl App for Touch {
    type Output = ();
    type Input = TouchInput;

    fn new() -> Self {
        Self {}
    }

    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, input: I, system: &System) -> Resul<Self::Output> {
        let i = TouchInput::deserialize(input).map_err(Erro::from_deserialize)?;
        system.run_args("/bin/touch", &[i.path]).await.map(|_| ())
    }
}

#[derive(Clone, Default)]
pub(crate) struct TouchBuilder;

impl AppBuilder for TouchBuilder {
    app_metadata!(
        Touch,
        "touch",
        "Touch command",
        &[Os::LinuxAny],
        AppExample::new("Run command",
            Box::new(TouchInput {
                path: "/tmp/file.txt".into()
            }),
            Box::new(())
        )
    );
}

#[cfg(test)]
mod test {
    use serde_json::to_value;
    use crate::apps::App;
    use crate::apps::touch::{Touch, TouchInput};
    use crate::utils::test::{system_user};

    #[tokio::test]
    async fn test_touch() {
        let path = "/tmp/test123";
        let system = system_user().await;
        Touch {}.run(to_value(TouchInput { path: path.into() }).unwrap(),
                     &system,
        ).await.unwrap();

        assert_eq!(system.read_to_string(path).await.unwrap(), "");
    }
}
