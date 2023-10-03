use std::collections::HashMap;
use std::num::ParseIntError;
use crate::files::prelude::*;
use thiserror::Error;

#[derive(Debug, Serialize, PartialEq, Description)]
pub(crate) struct CryptoItem {
    name: String,
    driver: String,
    module: String,
    priority: usize,
    refcnt: usize,
    selftest: String,
    internal: bool,
    r#type: String,
    blocksize: Option<usize>,
    digestsize: Option<usize>,
}

impl TryFrom<&str> for CryptoItem {
    type Error = CryptoError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();

        for line in value.split('\n').collect::<Vec<&str>>() {
            let kv = line.split(':').collect::<Vec<&str>>();

            if kv.len() != 2 {
                return Err(Self::Error::ItemKeyValue);
            }

            map.insert(kv[0].trim(), kv[1].trim().to_string());
        }

        Ok(Self {
            name: map.remove("name").ok_or(Self::Error::ItemKeyMissing)?,
            driver: map.remove("driver").ok_or(Self::Error::ItemKeyMissing)?,
            module: map.remove("module").ok_or(Self::Error::ItemKeyMissing)?,
            priority: map.remove("priority").ok_or(Self::Error::ItemKeyMissing)?.parse()?,
            refcnt: map.remove("refcnt").ok_or(Self::Error::ItemKeyMissing)?.parse()?,
            selftest: map.remove("selftest").ok_or(Self::Error::ItemKeyMissing)?,
            internal: map.remove("internal").ok_or(Self::Error::ItemKeyMissing)? == "yes",
            r#type: map.remove("type").ok_or(Self::Error::ItemKeyMissing)?,
            blocksize: map.remove("blocksize").map(|s| s.parse()).transpose()?,
            digestsize: map.remove("digestsize").map(|s| s.parse()).transpose()?,
        })
    }
}

pub(crate) struct Crypto;

impl Crypto {
    async fn parse(content: &str) -> Resul<Vec<CryptoItem>> {
        content.split("\n\n")
            .filter(|s| !s.is_empty())
            .map(CryptoItem::try_from)
            .collect::<Result<Vec<CryptoItem>, CryptoError>>()
            .map_err(Into::into)
    }
}

pub(crate) struct CryptoFile {
    path: String,
}

#[async_trait]
impl File for CryptoFile {
    type Output = Vec<CryptoItem>;
    type Input = ();

    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
        }
    }

    async fn read(&self, system: &System) -> Resul<Self::Output> {
        Crypto::parse(&system.read_to_string(self.path()).await?).await
    }
    fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone)]
pub(crate) struct CryptoBuilder;

impl FileBuilder for CryptoBuilder {
    type File = CryptoFile;

    const NAME: &'static str = "crypto";
    const DESCRIPTION: &'static str = "Get crypto information";
    const CAPABILITIES: &'static [Capability] = &[Capability::Read];

    fn patterns(&self) -> &[FileMatchPattern] {
        lazy_static! {
            static ref EXAMPLE: [FileMatchPattern; 1] = [FileMatchPattern::new_path("/proc/crypto",  &[Os::LinuxAny])];
        }

        EXAMPLE.as_ref()
    }

    fn examples(&self) -> &[FileExample] {
        lazy_static! {
            static ref EXAMPLE: [FileExample; 1] = [FileExample::new_get(
                "crypto details",
                vec![
                    CryptoItem {
                        name         : "crct10dif".into(),
                        driver       : "crct10dif-pclmul".into(),
                        module       : "crct10dif_pclmul".into(),
                        priority     : 200,
                        refcnt       : 2,
                        selftest     : "passed".into(),
                        internal     : false,
                        r#type         : "shash".into(),
                        blocksize    : Some(1),
                        digestsize   : Some(2),
                    }
                ]
            )];
        }

        EXAMPLE.as_ref()
    }
}

#[derive(Debug, Error)]
pub(crate) enum CryptoError {
    #[error("failed to parse value")]
    ItemKeyValue,
    #[error("failed to parse key")]
    ItemKeyMissing,
    #[error("failed to parse {0}")]
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for CryptoError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

#[cfg(test)]
mod test {
    use crate::files::crypto::{Crypto, CryptoItem};
    use crate::utils::test::{read_test_resources};

    #[tokio::test]
    async fn test_parse() {
        assert_eq!(Crypto::parse(&read_test_resources("crypto")).await.unwrap(),
                   vec![
                       CryptoItem { name: "__gcm(aes)".into(), driver: "__generic-gcm-aesni".into(), module: "aesni_intel".into(), priority: 400, refcnt: 1, selftest: "passed".into(), internal: true, r#type: "aead".into(), blocksize: Some(1), digestsize: None },
                       CryptoItem { name: "__ctr(aes)".into(), driver: "__ctr-aes-aesni".into(), module: "aesni_intel".into(), priority: 400, refcnt: 1, selftest: "passed".into(), internal: true, r#type: "skcipher".into(), blocksize: Some(1), digestsize: None },
                       CryptoItem { name: "__cts(cbc(aes))".into(), driver: "__cts-cbc-aes-aesni".into(), module: "aesni_intel".into(), priority: 400, refcnt: 1, selftest: "passed".into(), internal: true, r#type: "skcipher".into(), blocksize: Some(16), digestsize: None },
                       CryptoItem { name: "dh".into(), driver: "dh-generic".into(), module: "kernel".into(), priority: 100, refcnt: 1, selftest: "passed".into(), internal: false, r#type: "kpp".into(), blocksize: None, digestsize: None },
                   ]
        );
    }
}