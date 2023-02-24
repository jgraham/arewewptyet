use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Default)]
pub struct Metadata {
    host: String,
    products: Vec<String>,
}

impl Metadata {
    pub fn new(host: String) -> Metadata {
        Metadata {
            host,
            ..Default::default()
        }
    }

    pub fn add_product(&mut self, name: &str) {
        self.products.push(name.into());
    }

    pub fn url(&mut self) -> Url {
        let mut url = Url::parse(&format!("https://{}/api/metadata", self.host)).unwrap();
        {
            let mut query = url.query_pairs_mut();
            for product in self.products.iter() {
                query.append_pair("product", product);
            }
            query.append_pair("includeTestLevel", "true");
        }
        url
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataEntry {
    pub product: String,
    pub url: String,
    #[serde(default)]
    pub results: Vec<MetadataResult>,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataResult {
    pub subtest: Option<String>,
    // TODO: this should map onto a Status
    pub status: Option<u64>,
}

pub fn parse(json: &str) -> Result<BTreeMap<String, Vec<MetadataEntry>>, Error> {
    Ok(serde_json::from_str(json)?)
}
