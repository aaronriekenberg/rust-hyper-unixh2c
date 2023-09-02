use anyhow::Context;

use tokio::{sync::OnceCell, time::Duration};

use tracing::debug;

use std::{fmt::Debug, time::SystemTime};

use crate::config::StaticFileCacheRuleType;

static RULES_SERVICE_INSTANCE: OnceCell<StaticFileRulesService> = OnceCell::const_new();

trait CacheRule: Send + Sync + Debug {
    fn matches(&self, resolved_file: &hyper_staticfile::ResolvedFile) -> bool;

    fn build_cache_header(&self, resolved_file: &hyper_staticfile::ResolvedFile) -> Duration;
}

#[derive(Debug)]
struct FixedTimeCacheHeaderRule {
    url_regex: regex::Regex,
    file_cache_duration: Duration,
}

impl FixedTimeCacheHeaderRule {
    fn new(url_regex: regex::Regex, file_cache_duration: Duration) -> Self {
        Self {
            url_regex,
            file_cache_duration,
        }
    }
}

impl CacheRule for FixedTimeCacheHeaderRule {
    fn matches(&self, resolved_file: &hyper_staticfile::ResolvedFile) -> bool {
        let str_path = resolved_file.path.to_str().unwrap_or_default();

        self.url_regex.is_match(str_path)
    }

    fn build_cache_header(&self, _: &hyper_staticfile::ResolvedFile) -> Duration {
        self.file_cache_duration
    }
}

#[derive(Debug)]
struct ModificationTimePlusDeltaCacheHeaderRule {
    url_regex: regex::Regex,
    file_cache_duration: Duration,
}

impl ModificationTimePlusDeltaCacheHeaderRule {
    fn new(url_regex: regex::Regex, file_cache_duration: Duration) -> Self {
        Self {
            url_regex,
            file_cache_duration,
        }
    }
}

impl CacheRule for ModificationTimePlusDeltaCacheHeaderRule {
    fn matches(&self, resolved_file: &hyper_staticfile::ResolvedFile) -> bool {
        let str_path = resolved_file.path.to_str().unwrap_or_default();

        self.url_regex.is_match(str_path)
    }

    fn build_cache_header(&self, resolved_file: &hyper_staticfile::ResolvedFile) -> Duration {
        match resolved_file.modified {
            None => Duration::from_secs(0),
            Some(modified) => {
                let now = SystemTime::now();

                let file_expiration = modified + self.file_cache_duration;

                let request_cache_duration =
                    file_expiration.duration_since(now).unwrap_or_default();

                debug!(
                    "file_expiration = {:?} cache_duration = {:?}",
                    file_expiration, request_cache_duration
                );

                request_cache_duration
            }
        }
    }
}

#[derive(Debug)]
pub struct StaticFileRulesService {
    cache_rules: Vec<Box<dyn CacheRule>>,
}

impl StaticFileRulesService {
    fn new() -> anyhow::Result<Self> {
        let static_file_configuration = crate::config::instance().static_file_configuration();

        let mut cache_rules: Vec<Box<dyn CacheRule>> =
            Vec::with_capacity(static_file_configuration.cache_rules().len());

        for cache_rule in static_file_configuration.cache_rules() {
            let url_regex = regex::Regex::new(cache_rule.url_regex())
                .context("StaticFileRulesService::new: error parsing regex")?;

            match cache_rule.rule_type() {
                StaticFileCacheRuleType::FixedTime => {
                    cache_rules.push(Box::new(FixedTimeCacheHeaderRule::new(
                        url_regex,
                        cache_rule.duration(),
                    )));
                }
                StaticFileCacheRuleType::ModTimePlusDelta => {
                    cache_rules.push(Box::new(ModificationTimePlusDeltaCacheHeaderRule::new(
                        url_regex,
                        cache_rule.duration(),
                    )));
                }
            }
        }

        debug!("cache_rules = {:?}", cache_rules,);

        Ok(Self { cache_rules })
    }

    pub fn build_cache_header(
        &self,
        resolved_file: &hyper_staticfile::ResolvedFile,
    ) -> Option<Duration> {
        self.cache_rules
            .iter()
            .find(|rule| rule.matches(resolved_file))
            .map(|rule| rule.build_cache_header(resolved_file))
    }
}

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
