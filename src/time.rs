use chrono::prelude::{DateTime, Local, SecondsFormat};

pub type LocalDateTime = DateTime<Local>;

pub fn current_local_date_time() -> LocalDateTime {
    Local::now()
}

pub fn local_date_time_to_string(local_date_time: &LocalDateTime) -> String {
    local_date_time.to_rfc3339_opts(SecondsFormat::Millis, false)
}

pub fn current_local_date_time_string() -> String {
    local_date_time_to_string(&current_local_date_time())
}
