use anyhow::Context;

use getset::Getters;

use tracing::info;

use serde::{Deserialize, Serialize};

use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell};

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct ContextConfiguration {
    context: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ServerProtocol {
    #[serde(rename = "HTTP1")]
    Http1,

    #[serde(rename = "HTTP2")]
    Http2,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ServerSocketType {
    #[serde(rename = "TCP")]
    Tcp,

    #[serde(rename = "UNIX")]
    Unix,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct ServerConfiguration {
    server_protocol: ServerProtocol,
    server_socket_type: ServerSocketType,
    bind_address: String,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CommandInfo {
    id: String,
    description: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CommandConfiguration {
    max_concurrent_commands: usize,
    #[serde(with = "humantime_serde")]
    semaphore_acquire_timeout: std::time::Duration,
    commands: Vec<CommandInfo>,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct StaticFileConfiguration {
    path: String,
    precompressed_br: bool,
    precompressed_gz: bool,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct Configuration {
    context_configuration: ContextConfiguration,
    server_configurations: Vec<ServerConfiguration>,
    command_configuration: CommandConfiguration,
    static_file_configuration: StaticFileConfiguration,
}

static CONFIGURATION_INSTANCE: OnceCell<Configuration> = OnceCell::const_new();

pub async fn read_configuration(config_file: String) -> anyhow::Result<()> {
    info!("reading '{}'", config_file);

    let mut file = File::open(&config_file)
        .await
        .with_context(|| format!("error opening '{}'", config_file))?;

    let mut file_contents = Vec::new();

    file.read_to_end(&mut file_contents)
        .await
        .with_context(|| format!("error reading '{}'", config_file))?;

    let configuration: Configuration = ::serde_json::from_slice(&file_contents)
        .with_context(|| format!("error unmarshalling '{}'", config_file))?;

    info!("configuration\n{:#?}", configuration);

    CONFIGURATION_INSTANCE
        .set(configuration)
        .context("CONFIGURATION_INSTANCE.set error")?;

    Ok(())
}

pub fn instance() -> &'static Configuration {
    CONFIGURATION_INSTANCE.get().unwrap()
}
