use crate::error::Error;
use crate::result::Run;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Default)]
pub struct Runs {
    host: String,
    products: Vec<String>,
    labels: Vec<String>,
    max_count: Option<i64>,
}

impl Runs {
    pub fn new(host: String) -> Runs {
        Runs {
            host,
            ..Default::default()
        }
    }

    pub fn add_product(&mut self, name: &str, channel: &str) {
        self.products.push(format!("{}[{}]", name, channel));
    }

    pub fn add_label(&mut self, label: &str) {
        self.labels.push(label.into());
    }

    pub fn set_max_count(&mut self, max_count: i64) {
        self.max_count = Some(max_count);
    }

    pub fn url(&mut self) -> Url {
        let mut url = Url::parse(&format!("https://{}/api/runs", self.host)).unwrap();
        {
            let mut query = url.query_pairs_mut();
            for product in self.products.iter() {
                query.append_pair("product", product);
            }
            for label in self.labels.iter() {
                query.append_pair("label", label);
            }
            if let Some(count) = self.max_count {
                query.append_pair("max-count", &format!("{}", count));
            }
        }
        url
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
