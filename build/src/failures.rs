use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use time::serde::iso8601;
use time::OffsetDateTime;
use wptfyi::result::{Run, SearchData, Status};
use wptfyi::run;
use wptfyi::search::{AndClause, Clause, LinkClause, NotClause, OrClause, Query, ResultClause};

fn fx_only_failures_query(untriaged: bool) -> Query {
    let mut and_parts = Vec::new();

    let pass_statuses = vec![Status::Ok, Status::Pass];
    let pass_browsers = vec!["chrome", "safari"];

    for status in pass_statuses.iter() {
        and_parts.push(Clause::Not(NotClause {
            not: Box::new(Clause::Result(ResultClause {
                browser_name: "firefox".to_owned(),
                status: status.clone(),
            })),
        }));
    }
    for browser in pass_browsers {
        let mut or_parts = Vec::new();
        for status in pass_statuses.iter() {
            or_parts.push(Clause::Result(ResultClause {
                browser_name: browser.to_owned(),
                status: status.clone(),
            }));
        }
        and_parts.push(Clause::Or(OrClause { or: or_parts }));
    }

    if untriaged {
        and_parts.push(Clause::Not(NotClause {
            not: Box::new(Clause::Link(LinkClause {
                link: "bugzilla.mozilla.org".to_owned(),
            })),
        }))
    }

    Query {
        query: Clause::And(AndClause { and: and_parts }),
    }
}

pub fn get_runs(runs: &[Run]) -> Result<Vec<NewRun>> {
    let mut runs_by_commit = run::runs_by_commit(runs);
    let mut rv = Vec::with_capacity(runs_by_commit.len());
    for run in runs.iter() {
        if let Some(runs_for_rev) = runs_by_commit.remove(&run.full_revision_hash) {
            if runs_for_rev.len() == 3 {
                let date = runs_for_rev
                    .iter()
                    .map(|x| x.created_at)
                    .min()
                    .expect("No minimum found");
                let run_ids = runs_for_rev.iter().map(|x| x.id).collect();
                rv.push(NewRun {
                    revision: run.full_revision_hash.clone(),
                    run_ids,
                    date,
                })
            };
        }
    }
    Ok(rv)
}

pub struct NewRun {
    revision: String,
    run_ids: Vec<i64>,
    date: OffsetDateTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunsData {
    runs: Vec<RunData>,
}

impl RunsData {
    fn new() -> RunsData {
        RunsData { runs: Vec::new() }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RunData {
    revision: String,
    run_ids: Vec<i64>,
    #[serde(with = "iso8601")]
    date: OffsetDateTime,
    all_failures: FailureCount,
    untriaged_failures: FailureCount,
}

#[derive(Debug, Deserialize, Serialize)]
struct FailureCount {
    tests: i64,
    subtests: i64,
}

fn missing_runs(existing_runs: &RunsData, new_runs: Vec<NewRun>) -> Vec<NewRun> {
    let mut have_runs = HashSet::new();
    for existing_run in existing_runs.runs.iter() {
        have_runs.insert(existing_run.revision.clone());
    }
    new_runs
        .into_iter()
        .filter(|x| !have_runs.contains(&x.revision))
        .collect()
}

fn count_failures(failures_str: &str) -> Result<FailureCount> {
    let results: SearchData = serde_json::from_str(failures_str)?;
    let test_count = results.results.len() as i64;
    let subtest_count = results
        .results
        .iter()
        .map(|result| result.legacy_status.get(0).map(|x| x.total).unwrap_or(0))
        .sum();
    Ok(FailureCount {
        tests: test_count,
        subtests: subtest_count,
    })
}

pub mod update {
    use super::{
        count_failures, fx_only_failures_query, get_runs, missing_runs, RunData, RunsData,
    };
    use crate::network::{self, get, post};
    use anyhow::Result;
    use reqwest;
    use std::fs::File;
    use std::path::Path;
    use wptfyi::{result, run, Wptfyi};

    fn get_run_data(client: &reqwest::blocking::Client) -> Result<Vec<result::Run>> {
        let mut runs = Wptfyi::new(None).runs();
        for product in ["chrome", "firefox", "safari"].iter() {
            runs.add_product(product, "experimental")
        }
        runs.add_label("master");
        runs.set_max_count(100);
        Ok(run::parse(&get(client, &String::from(runs.url()), None)?)?)
    }

    pub fn get_fx_only_failures(
        client: &reqwest::blocking::Client,
        run_ids: &[i64],
        untriaged: bool,
    ) -> Result<String> {
        let mut search = Wptfyi::new(None).search();
        for product in ["chrome", "firefox", "safari"].iter() {
            search.add_product(product, "experimental")
        }
        search.set_query(run_ids, fx_only_failures_query(untriaged));
        search.add_label("master");
        post(client, &String::from(search.url()), None, search.body())
    }

    pub fn load_runs_data(path: &Path) -> Result<RunsData> {
        if let Ok(f) = File::open(path) {
            Ok(serde_json::from_reader(f)?)
        } else {
            Ok(RunsData::new())
        }
    }

    pub fn run() -> Result<()> {
        let client = network::client();

        let data_path = Path::new("../docs/runs.json");
        let mut runs_data = load_runs_data(data_path)?;

        let runs = get_run_data(&client)?;
        let runs = get_runs(&runs)?;
        let missing = missing_runs(&runs_data, runs);
        for new_run in missing.into_iter().rev() {
            let failures_all_str = match get_fx_only_failures(&client, &new_run.run_ids, false) {
                Ok(x) => x,
                Err(_) => continue,
            };
            let failures_untriaged_str = match get_fx_only_failures(&client, &new_run.run_ids, true)
            {
                Ok(x) => x,
                Err(_) => continue,
            };
            let count_all = count_failures(&failures_all_str)?;
            let count_untriaged = count_failures(&failures_untriaged_str)?;
            runs_data.runs.push(RunData {
                revision: new_run.revision,
                run_ids: new_run.run_ids,
                date: new_run.date,
                all_failures: count_all,
                untriaged_failures: count_untriaged,
            })
        }

        let out_f = File::create(data_path)?;
        serde_json::to_writer(out_f, &runs_data)?;
        Ok(())
    }
}
