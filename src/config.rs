use anyhow::Context;

use getset::{CopyGetters, Getters};

use tracing::debug;

use serde::{Deserialize, Serialize};

use tokio::{fs::File, io::AsyncReadExt, sync::OnceCell, time::Duration};

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct ContextConfiguration {
    dynamic_route_context: String,
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

#[derive(Debug, Deserialize, Serialize, CopyGetters, Getters)]
pub struct ServerListenerConfiguration {
    #[getset(get_copy = "pub")]
    server_protocol: ServerProtocol,

    #[getset(get_copy = "pub")]
    server_socket_type: ServerSocketType,

    #[getset(get = "pub")]
    bind_address: String,
}

#[derive(Debug, Deserialize, Serialize, CopyGetters, Getters)]
pub struct ServerConfiguration {
    #[getset(get = "pub")]
    listeners: Vec<ServerListenerConfiguration>,

    #[getset(get_copy = "pub")]
    connection_limit: usize,

    #[getset(get_copy = "pub")]
    #[serde(with = "humantime_serde")]
    connection_max_lifetime: Duration,

    #[getset(get_copy = "pub")]
    #[serde(with = "humantime_serde")]
    connection_graceful_shutdown_timeout: Duration,
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

#[derive(Debug, Deserialize, Serialize, CopyGetters, Getters)]
pub struct CommandConfiguration {
    #[getset(get_copy = "pub")]
    max_concurrent_commands: usize,

    #[getset(get_copy = "pub")]
    #[serde(with = "humantime_serde")]
    semaphore_acquire_timeout: Duration,

    #[getset(get = "pub")]
    commands: Vec<CommandInfo>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum StaticFileCacheRuleType {
    #[serde(rename = "MOD_TIME_PLUS_DELTA")]
    ModTimePlusDelta,

    #[serde(rename = "FIXED_TIME")]
    FixedTime,
}

#[derive(Debug, Deserialize, Serialize, CopyGetters, Getters)]
pub struct StaticFileCacheRule {
    #[getset(get = "pub")]
    path_regex: String,

    #[getset(get_copy = "pub")]
    rule_type: StaticFileCacheRuleType,

    #[getset(get_copy = "pub")]
    #[serde(with = "humantime_serde")]
    duration: Duration,
}

#[derive(Debug, Deserialize, Serialize, CopyGetters, Getters)]
pub struct StaticFileConfiguration {
    #[getset(get = "pub")]
    path: String,

    #[getset(get_copy = "pub")]
    precompressed_br: bool,

    #[getset(get_copy = "pub")]
    precompressed_gz: bool,

    #[getset(get = "pub")]
    client_error_page_path: String,

    #[getset(get = "pub")]
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    client_error_page_cache_duration: Option<Duration>,

    #[getset(get = "pub")]
    cache_rules: Vec<StaticFileCacheRule>,
}

#[derive(Debug, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct Configuration {
    context_configuration: ContextConfiguration,
    server_configuration: ServerConfiguration,
    command_configuration: CommandConfiguration,
    static_file_configuration: StaticFileConfiguration,
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
