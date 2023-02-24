use serde::{Deserialize, Serialize};
use time::serde::iso8601;
use time::OffsetDateTime;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Ok,
    Pass,
    Fail,
    Error,
    Timeout,
    NotRun,
    Crash,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Run {
    pub id: i64,
    pub browser_name: String,
    pub browser_version: String,
    pub os_name: String,
    pub os_version: String,
    pub revision: String,
    pub full_revision_hash: String,
    pub results_url: String,
    #[serde(with = "iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "iso8601")]
    pub time_start: OffsetDateTime,
    #[serde(with = "iso8601")]
    pub time_end: OffsetDateTime,
    pub raw_results_url: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchData {
    pub runs: Vec<Run>,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResult {
    pub test: String,
    pub legacy_status: Vec<LegacyStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LegacyStatus {
    pub passes: i64,
    pub total: i64,
}
