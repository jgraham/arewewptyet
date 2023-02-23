use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct YearData {
    pub table_sections: Vec<TableSections>,
    #[serde(default)]
    pub investigation_scores: Vec<InvestigationScore>,
    //investigation_weight: f64,
    pub csv_url: String,
    pub summary_feature_name: String,
    #[serde(default)]
    pub issue_url: String,
    #[serde(default)]
    pub matrix_url: String,
    pub focus_areas: BTreeMap<String, FocusArea>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TableSections {
    pub name: String,
    pub rows: Vec<String>,
    pub score_as_group: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InvestigationScore {
    pub name: String,
    pub scores_over_time: Vec<InvestigationUpdate>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InvestigationUpdate {
    pub date: String,
    pub score: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FocusArea {
    pub description: String,
    pub mdn: String,
    pub spec: String,
    pub tests: String,
    #[serde(rename = "countsTowardScore")]
    pub counts_toward_score: bool,
}

#[derive(Debug, Default)]
pub struct InteropData {
    pub host: String,
}

impl InteropData {
    pub fn new(host: String) -> InteropData {
        InteropData { host }
    }

    pub fn url(&self) -> Url {
        // TODO: Return a proper result type here
        Url::parse(&format!(
            "https://{}/static/interop-data_v2.json",
            self.host
        ))
        .unwrap()
    }
}

pub fn parse(json: &str) -> Result<BTreeMap<String, YearData>, Error> {
    Ok(serde_json::from_str(json)?)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Categories {
    pub categories: Vec<Category>,
}

impl Categories {
    pub fn by_name(&self) -> BTreeMap<String, &Category> {
        let mut rv = BTreeMap::new();
        for category in self.categories.iter() {
            rv.insert(category.name.clone(), category);
        }
        rv
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Category {
    pub name: String,
    pub labels: Vec<String>,
}

pub struct CategoryData {}

impl CategoryData {
    pub fn new() -> CategoryData {
        CategoryData {}
    }

    pub fn url(&self) -> Url {
        // TODO: Return a proper result type here
        Url::parse(
            "https://raw.githubusercontent.com/web-platform-tests/results-analysis/main/interop-scoring/category-data.json",
        )
        .unwrap()
    }
}

pub fn parse_categories(json: &str) -> Result<BTreeMap<String, Categories>, Error> {
    Ok(serde_json::from_str(json)?)
}
