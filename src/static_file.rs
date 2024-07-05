use anyhow::Context;

use tokio::{sync::OnceCell, time::Duration};

use tracing::debug;

use std::{fmt::Debug, time::SystemTime};

use crate::config::StaticFileCacheRuleType;

struct RequestMatchData<'a> {
    host_option: Option<&'a str>,
    resolved_path_option: Option<&'a str>,
}

#[derive(Debug)]
struct RequestMatcher {
    host_regex: Option<regex::Regex>,
    path_regex: Option<regex::Regex>,
}

impl RequestMatcher {
    fn new(cache_rule_config: &crate::config::StaticFileCacheRule) -> anyhow::Result<Self> {
        let host_regex = match &cache_rule_config.host_regex {
            None => None,
            Some(host_regex) => Some(
                regex::Regex::new(host_regex)
                    .context("StaticFileRulesService::new: error parsing host_regex")?,
            ),
        };

        let path_regex = match &cache_rule_config.path_regex {
            None => None,
            Some(path_regex) => Some(
                regex::Regex::new(path_regex)
                    .context("StaticFileRulesService::new: error parsing path_regex")?,
            ),
        };

        Ok(Self {
            host_regex,
            path_regex,
        })
    }

    fn matches(&self, request_match_data: &RequestMatchData) -> bool {
        let matches = match &self.host_regex {
            None => true,
            Some(host_regex) => match request_match_data.host_option {
                None => false,
                Some(host) => host_regex.is_match(host),
            },
        };

        if !matches {
            return false;
        }

        match &self.path_regex {
            None => true,
            Some(path_regex) => match request_match_data.resolved_path_option {
                None => false,
                Some(resolved_path) => path_regex.is_match(resolved_path),
            },
        }
    }
}

trait CacheRule: Send + Sync + Debug {
    fn build_cache_header(
        &self,
        resolved_file: &hyper_staticfile::ResolvedFile,
    ) -> Option<Duration>;
}

#[derive(Debug)]
struct FixedTimeCacheHeaderRule {
    file_cache_duration: Duration,
}

impl FixedTimeCacheHeaderRule {
    fn new(file_cache_duration: Duration) -> Self {
        Self {
            file_cache_duration,
        }
    }
}

impl CacheRule for FixedTimeCacheHeaderRule {
    fn build_cache_header(&self, _: &hyper_staticfile::ResolvedFile) -> Option<Duration> {
        Some(self.file_cache_duration)
    }
}

#[derive(Debug)]
struct ModificationTimePlusDeltaCacheHeaderRule {
    file_cache_duration: Duration,
}

impl ModificationTimePlusDeltaCacheHeaderRule {
    fn new(file_cache_duration: Duration) -> Self {
        Self {
            file_cache_duration,
        }
    }
}

impl CacheRule for ModificationTimePlusDeltaCacheHeaderRule {
    fn build_cache_header(
        &self,
        resolved_file: &hyper_staticfile::ResolvedFile,
    ) -> Option<Duration> {
        match resolved_file.modified {
            None => Some(Duration::from_secs(0)),
            Some(modified) => {
                let now = SystemTime::now();

                let file_expiration = modified + self.file_cache_duration;

                let request_cache_duration =
                    file_expiration.duration_since(now).unwrap_or_default();

                debug!(
                    "file_expiration = {:?} cache_duration = {:?}",
                    file_expiration, request_cache_duration
                );

                Some(request_cache_duration)
            }
        }
    }
}

#[derive(Debug)]
pub struct StaticFileRulesService {
    cache_rules: Vec<(RequestMatcher, Box<dyn CacheRule>)>,
}

impl StaticFileRulesService {
    fn new() -> anyhow::Result<Self> {
        let static_file_configuration = &crate::config::instance().static_file_configuration;

        let mut cache_rules: Vec<(RequestMatcher, Box<dyn CacheRule>)> =
            Vec::with_capacity(static_file_configuration.cache_rules.len());

        for cache_rule in &static_file_configuration.cache_rules {
            let request_matcher = RequestMatcher::new(cache_rule)?;

            match cache_rule.rule_type {
                StaticFileCacheRuleType::FixedTime => {
                    cache_rules.push((
                        request_matcher,
                        Box::new(FixedTimeCacheHeaderRule::new(cache_rule.duration)),
                    ));
                }
                StaticFileCacheRuleType::ModTimePlusDelta => {
                    cache_rules.push((
                        request_matcher,
                        Box::new(ModificationTimePlusDeltaCacheHeaderRule::new(
                            cache_rule.duration,
                        )),
                    ));
                }
            }
        }

        debug!("cache_rules = {:?}", cache_rules,);

        Ok(Self { cache_rules })
    }

    pub fn build_cache_header(
        &self,
        host_option: Option<&str>,
        resolved_file: &hyper_staticfile::ResolvedFile,
    ) -> Option<Duration> {
        let resolved_path_option = resolved_file.path.to_str();

        let request_match_data = RequestMatchData {
            host_option,
            resolved_path_option,
        };

        self.cache_rules
            .iter()
            .find(|(matcher, _)| matcher.matches(&request_match_data))
            .and_then(|(_, rule)| rule.build_cache_header(resolved_file))
    }
}

static RULES_SERVICE_INSTANCE: OnceCell<StaticFileRulesService> = OnceCell::const_new();

pub fn create_rules_service_instance() -> anyhow::Result<()> {
    let static_file_rules_service = StaticFileRulesService::new()?;

    RULES_SERVICE_INSTANCE
        .set(static_file_rules_service)
        .context("RULES_SERVICE_INSTANCE.set error")?;

    Ok(())
}

pub fn rules_service_instance() -> &'static StaticFileRulesService {
    RULES_SERVICE_INSTANCE.get().unwrap()
}
