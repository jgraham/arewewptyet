use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub created_at: DateTime<Utc>,
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
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
