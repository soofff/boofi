use crate::apps::prelude::*;
use thiserror::Error;
use crate::system::System;

pub(crate) enum UnameOptions {
    All,
    /*KernelName,
    Nodename,
    KernelRelease,
    KernelVersion,
    Machine,
    Processor,
    Hardwareplatform,
    OperatingSystem,*/
}

impl UnameOptions {
    pub(crate) fn value(&self) -> &str {
        match self {
            UnameOptions::All => "-a",
            /*UnameOptions::KernelName => "-s",
            UnameOptions::Nodename => "-n",
            UnameOptions::KernelRelease => "-r",
            UnameOptions::KernelVersion => "-v",
            UnameOptions::Machine => "-m",
            UnameOptions::Processor => "-p",
            UnameOptions::Hardwareplatform => "-i",
            UnameOptions::OperatingSystem => "-o",*/
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Description)]
pub(crate) struct Uname {
    kernel_name: String,
    nodename: String,
    kernel_release: String,
    kernel_version: String,
    machine: String,
    processor: String,
    hardware_platform: String,
    operating_system: String,
}

impl Uname {
    pub(crate) fn executable() -> &'static str { "/bin/uname" }
}

impl Uname {
    pub(crate) fn parse(content: &str) -> Resul<Uname> {
        let mut left: Vec<&str> = content.splitn(4, ' ').collect();
        let mut right: Vec<&str> = left.last().ok_or(UnameError::ParseRight)?.trim_end().rsplitn(5, ' ').collect();

        Ok(Self {
            kernel_name: left.remove(0).into(),
            nodename: left.remove(0).into(),
            kernel_release: left.remove(0).into(),
            operating_system: right.remove(0).into(),
            hardware_platform: right.remove(0).into(),
            processor: right.remove(0).into(),
            machine: right.remove(0).into(),
            kernel_version: right.remove(0).into(),
        })
    }
}

pub(crate) struct UnameApp {}

impl UnameApp {
    pub(crate) async fn run_parse(system: &System) -> Resul<Uname> {
        let o = system.run_args(Uname::executable(), &[UnameOptions::All.value()]).await?;
        Uname::parse(&String::from_utf8(o)?)
    }
}

#[async_trait]
impl App for UnameApp {
    type Output = Uname;
    type Input = ();

    fn new() -> Self {
        Self {}
    }

    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, _input: I, system: &System) -> Resul<Self::Output> {
        UnameApp::run_parse(system).await
    }
}

#[derive(Clone, Default)]
pub(crate) struct UnameBuilder;

impl AppBuilder for UnameBuilder {
    app_metadata!(
        UnameApp,
        "uname",
        "operating system information. currently -a supported",
        &[Os::LinuxAny],
        AppExample::new("get linux kernel information", Box::new(""), Box::new(Uname {
            kernel_name: "Linux".into(),
            nodename: "felix-VirtualBox".into(),
            kernel_release: "5.15.0-78-generic".into(),
            kernel_version: "#85~20.04.1-Ubuntu SMP Mon Jul 17 09:42:39 UTC 2023".into(),
            machine: "x86_64".into(),
            processor: "x86_64".into(),
            hardware_platform: "x86_64".into(),
            operating_system: "GNU/Linux".into(),
        }))
    );
}

#[derive(Debug, Error)]
pub(crate) enum UnameError {
    #[error("failed to parse from right")]
    ParseRight
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use crate::apps::App;
    use crate::apps::uname::{UnameApp};
    use crate::utils::test::{system_user};

    #[tokio::test]
    async fn test_run() {
        let result = UnameApp {}.run(json!(()), &system_user().await).await.unwrap();

        assert_eq!(result.kernel_name, "Linux");
        assert_eq!(result.hardware_platform, "x86_64");
    }
}