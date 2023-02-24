use crate::error::Error;
use crate::result::{SearchData, Status};
use serde::{Deserialize, Serialize};
use std::default::Default;
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct Query {
    pub query: Clause,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Clause {
    And(AndClause),
    Not(NotClause),
    Or(OrClause),
    Result(ResultClause),
    Link(LinkClause),
    Label(LabelClause),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AndClause {
    pub and: Vec<Clause>,
}

impl AndClause {
    pub fn push(&mut self, clause: Clause) {
        self.and.push(clause);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrClause {
    pub or: Vec<Clause>,
}

impl OrClause {
    pub fn push(&mut self, clause: Clause) {
        self.or.push(clause);
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NotClause {
    pub not: Box<Clause>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResultClause {
    pub browser_name: String,
    pub status: Status,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LinkClause {
    pub link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LabelClause {
    pub label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchBody {
    #[serde(flatten)]
    pub query: Query,
    pub run_ids: Vec<i64>,
}

#[derive(Debug, Default)]
pub struct Search {
    host: String,
    products: Vec<String>,
    labels: Vec<String>,
    body: Option<SearchBody>,
}

impl Search {
    pub fn new(host: String) -> Search {
        Search {
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

    pub fn url(&mut self) -> Url {
        let mut url = Url::parse(&format!("https://{}/api/search", self.host)).unwrap();
        {
            let mut query = url.query_pairs_mut();
            for product in self.products.iter() {
                query.append_pair("product", product);
            }
            for label in self.labels.iter() {
                query.append_pair("label", label);
            }
        }
        url
    }

    pub fn set_query(&mut self, run_ids: &[i64], query: Query) {
        self.body = Some(SearchBody {
            run_ids: run_ids.to_vec(),
            query,
        });
    }

    pub fn body(&self) -> Option<&SearchBody> {
        self.body.as_ref()
    }
}

pub fn parse(json: &str) -> Result<SearchData, Error> {
    Ok(serde_json::from_str(json)?)
}
