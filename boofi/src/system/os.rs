use std::str::FromStr;

use serde::Serialize;
use crate::error::Erro;

/// known (and unknown) operating systems
#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) enum Os {
    Unknown,
    LinuxUnknown,
    LinuxAny,
    LinuxArchlinux,
    LinuxFedora,
    LinuxOpenSusLeap,

    LinuxUbuntu,
    LinuxUbuntuLuna,
    LinuxUbuntuFocal,
    LinuxUbuntuBionic,

    LinuxDebian,
    LinuxDebianBookworm,
    LinuxDebianBullseye,
    LinuxDebianBuster,
}

impl Default for Os {
    fn default() -> Self {
        Self::Unknown
    }
}

impl FromStr for Os {
    type Err = Erro;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "linux" => Self::LinuxAny,
            "luna" => Self::LinuxUbuntuLuna,
            "focal" => Self::LinuxUbuntuFocal,
            "bionic" => Self::LinuxUbuntuBionic,
            "bookworm" => Self::LinuxDebianBookworm,
            "bullseye" => Self::LinuxDebianBullseye,
            "buster" => Self::LinuxDebianBuster,
            &_ => Self::Unknown
        })
    }
}

impl Os {
    pub(crate) fn compatible(&self, other: &Os) -> bool {
        if self == other {
            return true;
        }

        match self {
            Os::LinuxAny => [Os::LinuxArchlinux, Os::LinuxFedora, Os::LinuxOpenSusLeap,
                Os::LinuxDebian, Os::LinuxUbuntu, Os::LinuxUbuntuBionic, Os::LinuxUbuntuFocal,
                Os::LinuxUbuntuLuna, Os::LinuxDebianBookworm, Os::LinuxDebianBuster,
                Os::LinuxDebianBullseye].contains(other),
            Os::LinuxUbuntu => [Os::LinuxAny, Os::LinuxUbuntuBionic, Os::LinuxUbuntuFocal,
                Os::LinuxUbuntuLuna].contains(other),
            Os::LinuxDebian => [Os::LinuxAny, Os::LinuxDebianBookworm, Os::LinuxDebianBuster,
                Os::LinuxDebianBullseye].contains(other),
            _ => false,
        }
    }
}


#[cfg(test)]
mod test {
    use crate::system::os::Os;
    use crate::utils::test::{os};

    #[test]
    fn test_compatible() {
        assert!(Os::Unknown.compatible(&Os::Unknown));
        assert!(!Os::Unknown.compatible(&Os::LinuxAny));
        assert!(Os::LinuxAny.compatible(&Os::LinuxUbuntu));
        assert!(Os::LinuxUbuntu.compatible(&Os::LinuxAny));
        assert!(Os::LinuxUbuntu.compatible(&Os::LinuxUbuntuLuna));
        assert!(!Os::LinuxUbuntuLuna.compatible(&Os::LinuxUbuntu));
    }

    #[tokio::test]
    async fn test_supported() {
        let os = os().await;

        assert!(![Os::LinuxAny, Os::Unknown, Os::LinuxUnknown].contains(&os));
    }
}