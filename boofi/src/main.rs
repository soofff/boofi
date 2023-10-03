use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use crate::controller::Controller;
use crate::error::{Erro, Resul};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use tokio::fs::{File, read_to_string, write};
use std::str::FromStr;
use std::time::Duration;
use crate::rest::Rest;
use clap::Parser;


mod error;
mod rest;
mod files;
mod apps;
mod task;
mod utils;
mod system;
mod controller;
mod description;

/// Represents the SSL configuration
/// None:   ssl disabled
/// File:   certificates stored in files
/// Text:   certificates stored in configuration yaml
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SslConfig {
    None,
    File {
        private_key_path: String,
        certificate_path: String,
    },
    Text {
        private_key: String,
        certificate: String,
    },
}

impl Default for SslConfig {
    fn default() -> Self {
        Self::None
    }
}

/// Endpoint configuration
/// ssh:    service with ssh endpoint
/// local:  running service endpoint locally
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ServiceTypeConfig {
    Ssh {
        address: String
    },
    Local,
}

impl From<&ServiceTypeConfig> for Option<String> {
    fn from(value: &ServiceTypeConfig) -> Self {
        match value {
            ServiceTypeConfig::Local => None,
            ServiceTypeConfig::Ssh { address } => { Some(address.to_string()) }
        }
    }
}

/// General service configuration
/// name:   name is unique and describes the service path e.g. http://localhost/<name>/files
/// type:   service endpoint
#[derive(Debug, Serialize, Deserialize)]
struct ServiceConfig {
    name: String,
    r#type: ServiceTypeConfig,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "localhost".to_string(),
            r#type: ServiceTypeConfig::Local,
        }
    }
}

type Services = Vec<ServiceConfig>;

/// Represents the configuration file
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(skip)]
    path: String,
    listen: String,
    #[serde(serialize_with = "Config::serialize_duration", deserialize_with = "Config::deserialize_duration")]
    max_token_expiration: Duration,
    ssl: SslConfig,
    services: Services,
}

impl Config {
    fn serialize_duration<S: Serializer>(v: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(v.as_secs())
    }

    fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
        where
            D: Deserializer<'de>
    {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }

    async fn save(&self) -> Resul<()> {
        log::debug!("[SAVE] saving file to {}", self.path);
        let file = File::create(&self.path).await?;
        serde_yaml::to_writer(file.into_std().await, &self).map_err(Into::into)
    }

    async fn load_or_new(path: &str) -> Resul<Self> {
        if tokio::fs::try_exists(path).await? {
            log::debug!("[LOAD] loading file from {}", path);
            tokio::fs::read(path).await.map(|bytes| {
                serde_yaml::from_slice::<Config>(&bytes).map(|mut config| {
                    log::info!("[LOAD] configuration file loaded from {}", path);
                    config.path = path.into();
                    config
                })
            })?.map_err(Into::into)
        } else {
            log::debug!("[NEW] generate default config for {}", path);
            let this = Self {
                services: vec![Default::default()],
                path: path.into(),
                listen: "127.0.0.1:3000".into(),
                max_token_expiration: Duration::from_secs(60 * 60 * 24),
                ssl: Default::default(),
            };

            this.save().await?;
            log::info!("[NEW] configuration file saved to {}", path);

            Ok(this)
        }
    }

    async fn ssl(&self) -> Resul<Option<(String, String)>> {
        Ok(match &self.ssl {
            SslConfig::None => None,
            SslConfig::File { private_key_path, certificate_path } => {
                Some((read_to_string(private_key_path).await?,
                      read_to_string(certificate_path).await?
                ))
            }
            SslConfig::Text { private_key, certificate } => Some((private_key.into(), certificate.into()))
        })
    }
}

/// Command line options
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Path to config file", default_value = "./boofi.yml")]
    config: String,

    #[arg(long, help = "Generate self signed ssl. Can be used with --ssl_stored_file_path.")]
    self_signed_alt_names: Vec<String>,

    #[arg(long, help = "Directory location of self signed generated certificate and private key. Only usable with --self_signed_alt_names.")]
    ssl_stored_file_path: Option<String>,
}

#[tokio::main]
async fn main() -> Resul<()> {
    env_logger::init();

    let args = Args::parse();

    let mut config = Config::load_or_new(&args.config).await?;

    if args.self_signed_alt_names.is_empty() {
        log::debug!("starting rest api on {}", config.listen);
        let rest = Rest::new(SocketAddr::from_str(config.listen.as_str())?);
        let mut services = HashMap::new();

        for service_config in config.services.iter() {
            let name = service_config.name.clone();
            log::debug!("preparing service {}", name);
            let address: Option<String> = (&service_config.r#type).into();
            let service = rest.new_service(Controller::new(config.max_token_expiration,
                                                           address.as_deref()).await?).await;
            services.insert(service_config.name.clone(), service);
            log::debug!("service {} configured", name);
        }

        match config.ssl().await? {
            Some((private_key, certificate)) => rest.ssl(services, &private_key, &certificate).await?,
            None => rest.start(services).await.map_err(Into::<Erro>::into)?,
        }
    } else {
        let certs = rcgen::generate_simple_self_signed(args.self_signed_alt_names)?;
        log::info!("self signed certificate generated");

        let private_key = certs.serialize_private_key_pem();
        let certificate = certs.serialize_pem()?;

        if let Some(path) = args.ssl_stored_file_path {
            let priv_key_path = Path::new(&path).join("cert.key");
            let cert_path = Path::new(&path).join("cert.pem");

            let private_key_path = priv_key_path.to_str().ok_or(Erro::PrivateKeyPath)?.into();
            let certificate_path = cert_path.to_str().ok_or(Erro::CertificatePath)?.into();

            write(priv_key_path, private_key).await?;
            write(cert_path, certificate).await?;

            log::info!("key and certificate written to {}", path);

            config.ssl = SslConfig::File {
                private_key_path,
                certificate_path,
            }
        } else {
            config.ssl = SslConfig::Text {
                private_key,
                certificate,
            }
        }
        config.save().await?;
        log::info!("configuration updated");
    }
    Ok(())
}
