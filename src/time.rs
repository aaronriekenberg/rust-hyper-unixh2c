use chrono::prelude::{DateTime, Local, SecondsFormat};

pub fn current_local_date_time() -> DateTime<Local> {
    Local::now()
}

pub fn local_date_time_to_string(local_date_time: &DateTime<Local>) -> String {
    local_date_time.to_rfc3339_opts(SecondsFormat::Millis, false)
}

pub fn current_local_date_time_string() -> String {
    local_date_time_to_string(&current_local_date_time())
}
