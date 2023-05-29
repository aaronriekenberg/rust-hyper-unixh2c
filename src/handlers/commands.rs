use std::{path::PathBuf, sync::Arc};

use anyhow::Context;

use async_trait::async_trait;

use http_body_util::{BodyExt, Full};

use hyper::http::{Method, Response, StatusCode};

use tracing::warn;

use tokio::{
    process::Command,
    sync::{OnceCell, Semaphore, SemaphorePermit},
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    handlers::route::RouteInfo,
    handlers::utils::{build_json_body_response, build_json_response, build_status_code_response},
    handlers::{HttpRequest, RequestHandler, ResponseBody},
    response::CacheControl,
    time::current_local_date_time_string,
};

struct AllCommandsHandler;

impl AllCommandsHandler {
    async fn json_string() -> anyhow::Result<&'static str> {
        static INSTANCE: OnceCell<String> = OnceCell::const_new();

        let string = INSTANCE
            .get_or_try_init(|| async move {
                let commands = crate::config::instance().command_configuration().commands();
                serde_json::to_string(commands)
            })
            .await
            .context("AllCommandsHandler::json_string: INSTANCE.get_or_try_init error")?;

        Ok(string)
    }

    async fn instance() -> anyhow::Result<Self> {
        Self::json_string().await?;

        Ok(Self)
    }
}

#[async_trait]
impl RequestHandler for AllCommandsHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<ResponseBody> {
        let json_string = Self::json_string().await.unwrap();
        build_json_body_response(
            Full::from(json_string).map_err(|e| e.into()).boxed(),
            CacheControl::NoCache,
        )
    }
}

#[derive(thiserror::Error, Debug)]
enum RunCommandSemaporeAcquireError {
    #[error("acquire timeout: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("acquire error: {0}")]
    AcquireError(#[from] tokio::sync::AcquireError),
}

struct RunCommandSemapore {
    semapore: Semaphore,
    acquire_timeout: Duration,
}

impl RunCommandSemapore {
    fn new(command_configuration: &crate::config::CommandConfiguration) -> Arc<Self> {
        Arc::new(Self {
            semapore: Semaphore::new(*command_configuration.max_concurrent_commands()),
            acquire_timeout: *command_configuration.semaphore_acquire_timeout(),
        })
    }

    async fn acquire(&self) -> Result<SemaphorePermit<'_>, RunCommandSemaporeAcquireError> {
        let result = tokio::time::timeout(self.acquire_timeout, self.semapore.acquire()).await?;

        let permit = result?;

        Ok(permit)
    }
}

#[derive(Debug, Serialize)]
struct RunCommandResponse<'a> {
    now: String,
    command_duration_ms: u128,
    command_info: &'a crate::config::CommandInfo,
    command_output: String,
}

struct RunCommandHandler {
    run_command_semaphore: Arc<RunCommandSemapore>,
    command_info: &'static crate::config::CommandInfo,
}

impl RunCommandHandler {
    fn new(
        run_command_semaphore: Arc<RunCommandSemapore>,
        command_info: &'static crate::config::CommandInfo,
    ) -> Self {
        Self {
            run_command_semaphore,
            command_info,
        }
    }

    async fn run_command(&self) -> Result<std::process::Output, std::io::Error> {
        let output = Command::new(self.command_info.command())
            .args(self.command_info.args())
            .output()
            .await?;

        Ok(output)
    }

    fn handle_command_result(
        &self,
        command_result: Result<std::process::Output, std::io::Error>,
        command_duration: Duration,
    ) -> Response<ResponseBody> {
        let response = RunCommandResponse {
            now: current_local_date_time_string(),
            command_duration_ms: command_duration.as_millis(),
            command_info: self.command_info,
            command_output: match command_result {
                Err(err) => {
                    format!("error running command {}", err)
                }
                Ok(command_output) => {
                    let mut combined_output = String::with_capacity(
                        command_output.stderr.len() + command_output.stdout.len(),
                    );
                    combined_output.push_str(&String::from_utf8_lossy(&command_output.stderr));
                    combined_output.push_str(&String::from_utf8_lossy(&command_output.stdout));
                    combined_output
                }
            },
        };

        build_json_response(response, CacheControl::NoCache)
    }
}

#[async_trait]
impl RequestHandler for RunCommandHandler {
    async fn handle(&self, _request: &HttpRequest) -> Response<ResponseBody> {
        let run_command_permit = match self.run_command_semaphore.acquire().await {
            Err(err) => {
                warn!("run_command_semaphore.acquire error: {}", err);
                return build_status_code_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    CacheControl::NoCache,
                );
            }
            Ok(permit) => permit,
        };

        let command_start_time = Instant::now();
        let command_result = self.run_command().await;
        let command_duration = command_start_time.elapsed();

        drop(run_command_permit);

        self.handle_command_result(command_result, command_duration)
    }
}

pub async fn create_routes() -> anyhow::Result<Vec<RouteInfo>> {
    let command_configuration = crate::config::instance().command_configuration();

    let mut routes: Vec<RouteInfo> = Vec::with_capacity(1 + command_configuration.commands().len());

    routes.push(RouteInfo {
        method: &Method::GET,
        path_suffix: PathBuf::from("commands"),
        handler: Box::new(AllCommandsHandler::instance().await?),
    });

    let run_command_semaphore = RunCommandSemapore::new(command_configuration);

    for command_info in command_configuration.commands() {
        let path_suffix = PathBuf::from("commands").join(command_info.id());

        routes.push(RouteInfo {
            method: &Method::GET,
            path_suffix,
            handler: Box::new(RunCommandHandler::new(
                Arc::clone(&run_command_semaphore),
                command_info,
            )),
        });
    }

    Ok(routes)
}
