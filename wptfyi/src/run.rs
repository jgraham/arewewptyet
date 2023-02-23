use super::WptfyiUrl;
use crate::error::Error;
use crate::result::Run;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Default)]
pub struct Runs {
    host: String,
    url: WptfyiUrl,
}

impl Runs {
    pub fn new(host: String) -> Runs {
        Runs {
            host,
            ..Default::default()
        }
    }

    pub fn add_product(&mut self, name: &str, channel: &str) {
        self.url.add_product(name, channel);
    }

    pub fn add_label(&mut self, label: &str) {
        self.url.add_label(label);
    }

    pub fn set_max_count(&mut self, max_count: i64) {
        self.url.set_max_count(Some(max_count));
    }

    pub fn url(&mut self) -> Url {
        self.url.url(&self.host, "runs")
    }
}

pub fn parse(json: &str) -> Result<Vec<Run>, Error> {
    Ok(serde_json::from_str(json)?)
}

pub fn runs_by_commit(runs: &[Run]) -> HashMap<String, Vec<&Run>> {
    let mut runs_by_commit: HashMap<String, Vec<&Run>> = HashMap::new();
    for run in runs.iter() {
        runs_by_commit
            .entry(run.full_revision_hash.to_owned())
            .or_insert_with(|| vec![])
            .push(run);
    }
    runs_by_commit
}
