pub mod error;
pub mod interop;
pub mod result;
pub mod run;
pub mod search;

use std::default::Default;
use url::Url;

pub struct Wptfyi {
    pub host: String,
}

impl Wptfyi {
    pub fn new(host: Option<String>) -> Wptfyi {
        Wptfyi {
            host: host.unwrap_or_else(|| "wpt.fyi".into()),
        }
    }

    pub fn runs(&self) -> run::Runs {
        run::Runs::new(self.host.clone())
    }

    pub fn search(&self) -> search::Search {
        search::Search::new(self.host.clone())
    }

    pub fn interop_data(&self) -> interop::InteropData {
        interop::InteropData::new(self.host.clone())
    }

    pub fn interop_categories(&self) -> interop::CategoryData {
        interop::CategoryData::new()
    }
}

#[derive(Debug, Default)]
pub(crate) struct WptfyiUrl {
    products: Vec<String>,
    labels: Vec<String>,
    max_count: Option<i64>,
}

impl WptfyiUrl {
    pub(crate) fn add_product(&mut self, name: &str, channel: &str) {
        self.products.push(format!("{}[{}]", name, channel));
    }

    pub(crate) fn add_label(&mut self, label: &str) {
        self.labels.push(label.into())
    }

    pub(crate) fn set_max_count(&mut self, max_count: Option<i64>) {
        self.max_count = max_count
    }

    pub(crate) fn url(&mut self, host: &str, path: &str) -> Url {
        // TODO: Return a proper result type here
        let mut url = Url::parse(&format!("https://{}/api/{}", host, path)).unwrap();
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
