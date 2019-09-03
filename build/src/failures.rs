use chrono::{DateTime, Utc};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize, Serialize)]
struct Search {
    #[serde(flatten)]
    query: Query,
    run_ids: Vec<i64>
}

#[derive(Debug, Deserialize, Serialize)]
struct Query {
    query: Clause
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum Clause {
    And(AndClause),
    Not(NotClause),
    Or(OrClause),
    Result(ResultClause),
    Link(LinkClause)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Status {
    Ok,
    Pass,
    Fail,
    Error,
    Timeout,
    NotRun,
    Crash
}

#[derive(Debug, Deserialize, Serialize)]
struct AndClause {
    and: Vec<Clause>
}

impl AndClause {
    fn push(&mut self, clause: Clause) {
        self.and.push(clause);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct OrClause {
    or: Vec<Clause>
}

impl OrClause {
    fn push(&mut self, clause: Clause) {
        self.or.push(clause);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct NotClause {
    not: Box<Clause>
}

#[derive(Debug, Deserialize, Serialize)]
struct ResultClause {
    browser_name: String,
    status: Status
}

#[derive(Debug, Deserialize, Serialize)]
struct LinkClause {
    link: String
}

fn fx_only_failures_query(untriaged: bool) -> Query {
    let mut and_parts = Vec::new();

    let pass_statuses = vec!(Status::Ok, Status::Pass);
    let pass_browsers = vec!("chrome", "safari");

    for status in pass_statuses.iter() {
        and_parts.push(Clause::Not(
            NotClause {
                not: Box::new(
                    Clause::Result(
                        ResultClause {
                            browser_name: "firefox".to_owned(),
                            status: status.clone()
                        })
                )})
        );
    };
    for browser in pass_browsers {
        let mut or_parts = Vec::new();
        for status in pass_statuses.iter() {
            or_parts.push(
                Clause::Result(
                    ResultClause {
                        browser_name: browser.to_owned(),
                        status: status.clone()
                    })
            );
        }
        and_parts.push(Clause::Or(
            OrClause {
                or: or_parts
            }));
    }

    if untriaged {
        and_parts.push(Clause::Not(
            NotClause {
                not: Box::new(
                    Clause::Link(
                        LinkClause {
                            link: "bugzilla.mozilla.org".to_owned()
                        }
                    )
                )}))
    }

    Query {
        query: Clause::And(AndClause {
            and: and_parts
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Run {
    id: i64,
    browser_name: String,
    browser_version: String,
    os_name: String,
    os_version: String,
    revision: String,
    full_revision_hash: String,
    results_url: String,
    created_at: DateTime<Utc>,
    time_start: DateTime<Utc>,
    time_end: DateTime<Utc>,
    raw_results_url: String,
    labels: Vec<String>
}

pub fn get_runs(runs_json: &str) -> Result<Vec<NewRun>> {
    let runs: Vec<Run> = serde_json::from_str(runs_json)?;
    let mut rv = Vec::new();
    let mut runs_by_commit: HashMap<String, Vec<(i64, DateTime<Utc>)>> = HashMap::new();
    for run in runs.iter() {
        runs_by_commit
            .entry(run.full_revision_hash.to_owned())
            .or_insert_with(|| vec!())
            .push((run.id, run.created_at));
    }
    for run in runs.iter() {
        if let Some(runs_for_rev) = runs_by_commit.remove(&run.full_revision_hash) {
            if runs_for_rev.len() == 3 {
                let date = runs_for_rev.iter().map(|x| x.1).min().expect("No minimum found");
                let run_ids = runs_for_rev.iter().map(|x| x.0).collect();
                rv.push(NewRun {
                    revision: run.full_revision_hash.clone(),
                    run_ids,
                    date
                })
            };
        }
    }
    Ok(rv)
}

pub struct NewRun {
    revision: String,
    run_ids: Vec<i64>,
    date: DateTime<Utc>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunsData {
    runs: Vec<RunData>
}

impl RunsData {
    fn new() -> RunsData {
        RunsData {
            runs: Vec::new()
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RunData {
    revision: String,
    run_ids: Vec<i64>,
    date: DateTime<Utc>,
    all_failures: FailureCount,
    untriaged_failures: FailureCount
}

#[derive(Debug, Deserialize, Serialize)]
struct FailureCount {
    tests: i64,
    subtests: i64
}

fn missing_runs(existing_runs: &RunsData, new_runs: Vec<NewRun>) -> Vec<NewRun> {
    let mut have_runs = HashSet::new();
    for existing_run in existing_runs.runs.iter() {
        have_runs.insert(existing_run.revision.clone());
    }
    new_runs.into_iter().filter(|x| !have_runs.contains(&x.revision)).collect()
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchData {
    runs: Vec<Run>,
    results: Vec<SearchResult>
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchResult {
    test: String,
    legacy_status: Vec<LegacyStatus>
}

#[derive(Debug, Deserialize, Serialize)]
struct LegacyStatus {
    passes: i64,
    total: i64
}

fn count_failures(failures_str: &str) -> Result<FailureCount> {
    let results: SearchData = serde_json::from_str(&failures_str)?;
    let test_count = results.results.len() as i64;
    let subtest_count = results.results.iter().map(
        |result| result
            .legacy_status
            .get(0)
            .map(|x| x.total)
            .unwrap_or(0))
        .sum();
    Ok(FailureCount {
        tests: test_count,
        subtests: subtest_count
    })
}


pub mod update {
    use reqwest;
    use crate::network::{get, post};
    use crate::error::Result;
    use std::fs::File;
    use std::path::Path;
    use super::{get_runs, fx_only_failures_query, RunData, RunsData, missing_runs, count_failures, Search};

    fn get_run_data(client: &reqwest::Client) -> Result<String> {
        Ok(get(&client,
               "https://wpt.fyi/api/runs?label=master&product=chrome%5Bexperimental%5D&product=firefox%5Bexperimental%5D&product=safari%5Bexperimental%5D&max-count=100&aligned",
               None)?)
    }

    pub fn get_fx_only_failures(client: &reqwest::Client, run_ids: &Vec<i64>, untriaged: bool) -> Result<String> {
        let query = fx_only_failures_query(untriaged);
        let search = Search {
            query,
            run_ids: run_ids.clone()
        };
        post(&client,
             "https://wpt.fyi/api/search?label=master&product=chrome%5Bexperimental%5D&product=firefox%5Bexperimental%5D&product=safari%5Bexperimental%5D",
             None,
             Some(search))
    }

    pub fn load_runs_data(path: &Path) -> Result<RunsData> {
        if let Ok(f) = File::open(path) {
            Ok(serde_json::from_reader(f)?)
        } else {
            Ok(RunsData::new())
        }
    }

    pub fn run() -> Result<()> {
        let client = reqwest::Client::new();

        let data_path = Path::new("../docs/runs.json");
        let mut runs_data = load_runs_data(data_path)?;

        let runs_str = get_run_data(&client)?;
        let runs = get_runs(&runs_str)?;
        let missing = missing_runs(&runs_data, runs);
        for new_run in missing.into_iter().rev() {
            let failures_all_str = match get_fx_only_failures(&client, &new_run.run_ids, false) {
                Ok(x) => x,
                Err(_) => continue
            };
            let failures_untriaged_str = match get_fx_only_failures(&client, &new_run.run_ids, true) {
                Ok(x) => x,
                Err(_) => continue
            };
            let count_all = count_failures(&failures_all_str)?;
            let count_untriaged = count_failures(&failures_untriaged_str)?;
            runs_data.runs.push(RunData {
                revision: new_run.revision,
                run_ids: new_run.run_ids,
                date: new_run.date,
                all_failures: count_all,
                untriaged_failures: count_untriaged
            })
        }

        let out_f = File::create(data_path)?;
        serde_json::to_writer(out_f, &runs_data)?;
        Ok(())
    }
}
