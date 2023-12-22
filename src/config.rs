use anyhow::Context;

use tracing::debug;

use serde::{Deserialize, Serialize};

use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell, time::Duration};

#[derive(Debug, Deserialize, Serialize)]
pub struct ContextConfiguration {
    pub dynamic_route_context: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ServerSocketType {
    #[serde(rename = "TCP")]
    Tcp,

    #[serde(rename = "UNIX")]
    Unix,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerListenerConfiguration {
    pub socket_type: ServerSocketType,
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConnectionConfiguration {
    pub limit: usize,
    #[serde(with = "humantime_serde")]
    pub max_lifetime: Duration,
    #[serde(with = "humantime_serde")]
    pub graceful_shutdown_timeout: Duration,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfiguration {
    pub listeners: Vec<ServerListenerConfiguration>,
    pub connection: ServerConnectionConfiguration,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandInfo {
    pub id: String,
    pub description: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandConfiguration {
    pub max_concurrent_commands: usize,

    #[serde(with = "humantime_serde")]
    pub semaphore_acquire_timeout: Duration,

    pub commands: Vec<CommandInfo>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum StaticFileCacheRuleType {
    #[serde(rename = "MOD_TIME_PLUS_DELTA")]
    ModTimePlusDelta,

    #[serde(rename = "FIXED_TIME")]
    FixedTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaticFileCacheRule {
    pub path_regex: String,
    pub rule_type: StaticFileCacheRuleType,
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaticFilePrecompressedConfiguration {
    pub br: bool,
    pub gz: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaticFileConfiguration {
    pub root: String,
    pub precompressed: StaticFilePrecompressedConfiguration,
    pub client_error_page_path: String,
    pub cache_rules: Vec<StaticFileCacheRule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub server_configuration: ServerConfiguration,
    pub static_file_configuration: StaticFileConfiguration,
    pub context_configuration: ContextConfiguration,
    pub command_configuration: CommandConfiguration,
}

static CONFIGURATION_INSTANCE: OnceCell<Configuration> = OnceCell::const_new();

pub async fn read_configuration(config_file: String) -> anyhow::Result<()> {
    debug!("reading '{}'", config_file);

    let mut file = File::open(&config_file)
        .await
        .with_context(|| format!("error opening '{}'", config_file))?;

    let mut file_contents = Vec::new();

    file.read_to_end(&mut file_contents)
        .await
        .with_context(|| format!("error reading '{}'", config_file))?;

    let file_contents_string = String::from_utf8(file_contents)
        .with_context(|| format!("String::from_utf8 error reading '{}'", config_file))?;

    let configuration: Configuration = ::toml::from_str(&file_contents_string)
        .with_context(|| format!("error unmarshalling '{}'", config_file))?;

    debug!("configuration\n{:#?}", configuration);

    CONFIGURATION_INSTANCE
        .set(configuration)
        .context("CONFIGURATION_INSTANCE.set error")?;

    Ok(())
}

pub fn instance() -> &'static Configuration {
    CONFIGURATION_INSTANCE.get().unwrap()
}
