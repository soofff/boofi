pub(crate) mod ls;
pub(crate) mod wget;
pub(crate) mod sh;
pub(crate) mod touch;
pub(crate) mod uname;

pub(crate) use crate::apps::ls::LsBuilder;
pub(crate) use crate::apps::sh::ShBuilder;
pub(crate) use crate::apps::touch::TouchBuilder;
pub(crate) use crate::apps::uname::UnameBuilder;
pub(crate) use crate::apps::wget::WgetBuilder;

use crate::error::Resul;
use crate::system::os::Os;
use crate::system::System;
use async_trait::async_trait;
use serde::{Deserializer, Serialize};
use crate::description::{Description, DescriptionField};

/// Add `crate::apps::prelude::*` to your app. It provides all basic dependencies to make a new app.
pub(crate) mod prelude {
    pub(crate) use crate::utils::{app_metadata, count};
    pub(crate) use super::{AppExample, AppBuilder, App};
    pub(crate) use lazy_static::lazy_static;
    pub(crate) use serde::{Deserialize, Serialize, Deserializer};
    pub(crate) use async_trait::async_trait;
    pub(crate) use crate::error::*;
    pub(crate) use crate::system::os::*;
    pub(crate) use crate::description::*;
}

pub(crate) type Serializable = Box<dyn erased_serde::Serialize + Send + Sync>;

/// All related app information in one struct.
/// Used for end user documentation
#[derive(Serialize)]
pub(crate) struct AppHelp<'a> {
    name: &'static str,
    description: &'static str,
    compatible: bool,
    input: &'static DescriptionField,
    output: &'static DescriptionField,
    supported_os: &'static [Os],
    examples: &'a [AppExample],
}

/// An app example usage
/// Helpful for end user
#[derive(Serialize)]
pub(crate) struct AppExample {
    description: &'static str,
    input: Serializable,
    output: Serializable,
}

impl AppExample {
    pub(crate) fn new(description: &'static str, input: Serializable, output: Serializable) -> Self {
        Self {
            description,
            input,
            output,
        }
    }
}

#[async_trait]
pub(crate) trait App: Send + Sync {
    type Output: Serialize + Description;
    type Input: Description;

    fn new() -> Self;

    /// The actual `run` call. It will be called mostly once per instance.
    async fn run<'de, I: Deserializer<'de> + Send>(&mut self, input: I, system: &System) -> Resul<Self::Output>;

    fn input_meta() -> &'static DescriptionField {
        Self::Input::field()
    }

    fn output_meta() -> &'static DescriptionField {
        Self::Output::field()
    }
}


pub(crate) trait AppBuilder {
    type App: App;

    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const SUPPORTED_OS: &'static [Os];

    /// Describes all input parameters with name, type, optional and default value.
    /// Use `doc_` macros to produce common structure.
    fn input(&self) -> &'static DescriptionField {
        Self::App::input_meta()
    }

    /// Expected output definition with name and type.
    /// Use `doc_` macros to produce common structure.
    fn output(&self) -> &'static DescriptionField {
        Self::App::output_meta()
    }

    /// One ore more examples with `description`, `input` and `output`.
    fn examples(&self) -> &[AppExample] {
        &[]
    }

    /// Summary of all related information
    fn help(&self, os: &Os) -> AppHelp {
        AppHelp {
            name: Self::NAME,
            description: Self::DESCRIPTION,
            supported_os: Self::SUPPORTED_OS,
            input: self.input(),
            output: self.output(),
            examples: self.examples(),
            compatible: self.compatible(os),
        }
    }

    /// Returns compatibility with the target `os`.
    fn compatible(&self, os: &Os) -> bool {
        Self::SUPPORTED_OS
            .iter()
            .any(|o| o.compatible(os))
    }

    fn new_app(&self) -> Self::App {
        Self::App::new()
    }
}

macro_rules! app_builders {
    ($(
        $typ:tt
    ),*
    ) => {
        #[derive(Clone)]
        pub(crate) enum AppBuilders {
            $(
                $typ($typ),
            )*
        }

        impl AppBuilders {
            pub(crate) fn name(&self) -> &str {
                match self {
                    $( Self::$typ(_)  => $typ::NAME, )*
                }
            }

            pub(crate) fn help(&self, os: &Os) -> AppHelp {
                match self {
                    $( Self::$typ(i)  => i.help(os), )*
                }
            }

            pub(crate) fn compatible(&self, os: &Os) -> bool {
                match self {
                    $( Self::$typ(i)  => i.compatible(os), )*
                }
            }

            pub(crate) async fn run<'de, I: Deserializer<'de> + Send + Sync>(&mut self, input: I, system: &System) -> Resul<Box<dyn erased_serde::Serialize + Send>> {
                match self {
                    $(
                    Self::$typ(i)  => {
                        Ok(i.new_app().run(input, system).await.map(Box::new)?)
                    },
                    )*
                }
            }
        }
    }
}

app_builders!(
    LsBuilder,
    ShBuilder,
    TouchBuilder,
    UnameBuilder,
    WgetBuilder
);



