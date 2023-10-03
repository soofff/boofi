#[cfg(test)]
pub(crate) mod test {
    use std::fs::read_to_string;
    use crate::system::os::Os;
    use crate::system::{Credential, Platform, System, PlatformActions};
    use crate::system::posix::Posix;

    pub(crate) const RESOURCES: &str = "/resources/test/";
    pub(crate) const SSH_ENDPOINT: &str = "127.0.0.1:22";
    pub(crate) const USERNAME: &str = "dev";
    pub(crate) const PASSWORD: &str = "admin12345";

    pub(crate) fn test_resources(name: &str) -> String {
        let mut base = env!("CARGO_MANIFEST_DIR").to_string();
        base.push_str(RESOURCES);
        base.push_str(name);
        base
    }

    pub(crate) fn read_test_resources(name: &str) -> String {
        read_to_string(test_resources(name)).unwrap()
    }

    fn endpoint_some() -> Option<String> {
        Some(SSH_ENDPOINT.into())
    }

    fn credential() -> Credential {
        Credential::new(USERNAME, PASSWORD)
    }

    pub(crate) async fn os() -> Os {
        Posix::new(credential(),
                   endpoint_some(),
        ).detect_os().await.unwrap()
    }

    pub(crate) async fn system_ssh() -> System {
        System::new(Platform::Posix(
            Posix::new(credential(),
                       endpoint_some(),
            )
        ), Some(os().await))
    }

    pub(crate) async fn system_user() -> System {
        System::new(Platform::Posix(
            Posix::new(credential(), None)
        ), Some(os().await))
    }
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + crate::utils::count!($($xs)*));
}

/// Generates file builder metadata
macro_rules! file_metadata {
    (
        $file:tt,
        $name:expr,
        $description:expr,
        $capabilities:expr,
        $ (
            $examples:expr
        ),*
        ;
        $ (
            $patterns:expr
        ),*
    ) => {
        type File = $file;

        const NAME: &'static str = $name;
        const DESCRIPTION: &'static str = $description;
        const CAPABILITIES: &'static [Capability] = $capabilities;

        fn examples(&self) -> &[FileExample] {
            lazy_static! {
                static ref EAMPLES: [FileExample; count!($($examples)*)] = [
                    $(
                        $examples,
                    )*
                ];
            }

            return EAMPLES.as_slice();
        }

        fn patterns(&self) -> &[FileMatchPattern] {
            lazy_static! {
                static ref PATTERNS: [FileMatchPattern; count!($($patterns)*)] = [
                        $(
                            $patterns,
                        )*
                    ];
            }

            return PATTERNS.as_slice();
        }
    }
}

/// generates app metadata for builder
macro_rules! app_metadata {
    (
        $app:tt,
        $name:expr,
        $description:expr,
        $os:expr,
        $ (
            $value:expr
        ),*
    ) => {
        type App = $app;

        const NAME: &'static str = $name;
        const DESCRIPTION: &'static str = $description;
        const SUPPORTED_OS: &'static [Os] = $os;

        fn examples(&self) -> &[AppExample] {
            lazy_static! {
                static ref EXAMPLES: [AppExample; count!($($value)*)] = [
                    $(
                        $value,
                    )*
                ];
            }
            return EXAMPLES.as_slice();
        }
    }
}

pub(crate) use app_metadata;
pub(crate) use file_metadata;
pub(crate) use count;
