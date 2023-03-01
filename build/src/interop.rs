use crate::network::{self, get, post};
use anyhow::{anyhow, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use url::Url;
use wptfyi::interop::{Category, FocusArea};
use wptfyi::metadata::MetadataEntry;
use wptfyi::result::Status;
use wptfyi::search::{AndClause, Clause, LabelClause, NotClause, OrClause, Query, ResultClause};
use wptfyi::{interop, metadata, result, run, search, Wptfyi};

fn fx_failures_query(labels: &[&str]) -> Query {
    let pass_statuses = &[Status::Ok, Status::Pass];

    let mut root_clause = AndClause {
        and: Vec::with_capacity(3),
    };

    for status in pass_statuses.iter() {
        root_clause.push(Clause::Not(NotClause {
            not: Box::new(Clause::Result(ResultClause {
                browser_name: "firefox".to_owned(),
                status: status.clone(),
            })),
        }));
    }

    if !labels.is_empty() {
        let mut labels_clause = OrClause {
            or: Vec::with_capacity(labels.len()),
        };
        for label in labels {
            labels_clause.push(Clause::Label(LabelClause {
                label: (*label).into(),
            }));
        }
        root_clause.push(Clause::Or(labels_clause));
    }

    Query {
        query: Clause::And(root_clause),
    }
}

fn get_run_data(wptfyi: &Wptfyi, client: &reqwest::blocking::Client) -> Result<Vec<result::Run>> {
    let mut runs = wptfyi.runs();
    for product in ["chrome", "firefox", "safari"].iter() {
        runs.add_product(product, "experimental")
    }
    runs.add_label("master");
    runs.set_max_count(100);
    Ok(run::parse(&get(client, &String::from(runs.url()), None)?)?)
}

fn get_metadata(
    wptfyi: &Wptfyi,
    client: &reqwest::blocking::Client,
) -> Result<BTreeMap<String, Vec<MetadataEntry>>> {
    let mut metadata = wptfyi.metadata();
    for product in ["firefox"].iter() {
        metadata.add_product(product)
    }
    Ok(metadata::parse(&get(
        client,
        &String::from(metadata.url()),
        None,
    )?)?)
}

pub fn get_fx_failures(
    wptfyi: &Wptfyi,
    client: &reqwest::blocking::Client,
    run_ids: &[i64],
    labels: &[&str],
) -> Result<result::SearchData> {
    let mut search = wptfyi.search();
    for product in ["chrome", "firefox", "safari"].iter() {
        search.add_product(product, "experimental")
    }
    search.set_query(run_ids, fx_failures_query(labels));
    search.add_label("master");
    Ok(search::parse(&post(
        client,
        &String::from(search.url()),
        None,
        search.body(),
    )?)?)
}

pub fn get_interop_data(
    wptfyi: &Wptfyi,
    client: &reqwest::blocking::Client,
) -> Result<BTreeMap<String, interop::YearData>> {
    let runs = wptfyi.interop_data();
    Ok(interop::parse(&get(
        client,
        &String::from(runs.url()),
        None,
    )?)?)
}

pub fn get_interop_categories(
    wptfyi: &Wptfyi,
    client: &reqwest::blocking::Client,
) -> Result<BTreeMap<String, interop::Categories>> {
    Ok(interop::parse_categories(&get(
        client,
        &String::from(wptfyi.interop_categories().url()),
        None,
    )?)?)
}

pub fn get_interop_scores(
    wptfyi: &Wptfyi,
    client: &reqwest::blocking::Client,
    browser_channel: interop::BrowserChannel,
) -> Result<Vec<interop::ScoreRow>> {
    Ok(interop::parse_scores(&get(
        client,
        &String::from(wptfyi.interop_scores(browser_channel).url()),
        None,
    )?)?)
}

fn latest_runs(runs: &[result::Run]) -> Result<Vec<&result::Run>> {
    let mut runs_by_commit = run::runs_by_commit(runs);
    let latest_rev = runs_by_commit
        .iter()
        .filter(|(_, value)| value.len() == 3)
        .max_by(|(_, value_1), (_, value_2)| {
            let date_1 = value_1.iter().map(|x| x.created_at).max();
            let date_2 = value_2.iter().map(|x| x.created_at).max();
            date_1.cmp(&date_2)
        })
        .map(|(key, _)| key.clone());
    latest_rev
        .and_then(|x| runs_by_commit.remove(&x))
        .ok_or_else(|| anyhow!("Failed to find any complete runs"))
}

pub fn write_focus_area(
    fyi: &Wptfyi,
    client: &reqwest::blocking::Client,
    name: &str,
    focus_area: &FocusArea,
    run_ids: &[i64],
    categories_by_name: &BTreeMap<String, &Category>,
    metadata: &BTreeMap<String, Vec<MetadataEntry>>,
) -> Result<()> {
    if !focus_area.counts_toward_score {
        return Ok(());
    }
    let labels = &categories_by_name
        .get(name)
        .ok_or_else(|| anyhow!("Didn't find category {}", name))?
        .labels;
    let path = format!("../docs/interop-2023/{}.csv", name);
    let data_path = Path::new(&path);
    let out_f = File::create(data_path)?;
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(out_f);

    let results = get_fx_failures(
        &fyi,
        &client,
        &run_ids,
        &labels
            .iter()
            .filter_map(|x| {
                if x.starts_with("interop-") {
                    Some(x.as_ref())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>(),
    )?;
    let order = &["firefox", "chrome", "safari"];
    let maybe_browser_list = results
        .runs
        .iter()
        .map(|x| order.iter().position(|target| *target == x.browser_name))
        .collect::<Option<Vec<usize>>>();
    if maybe_browser_list.is_none() {
        return Err(anyhow!("Didn't get results for all three browsers"));
    }
    let browser_list = maybe_browser_list.unwrap();

    writer.write_record([
        "Test",
        "Firefox Failures",
        "Chrome Failures",
        "Safari Failures",
        "Bugs",
    ])?;
    for result in results.results.iter() {
        let mut scores = vec![String::new(), String::new(), String::new()];
        for (output_idx, browser_idx) in browser_list.iter().enumerate() {
            if let Some(status) = result.legacy_status.get(*browser_idx) {
                if output_idx == 0 {
                    // For Firefox output the total as this is the number of failures
                    scores[output_idx].push_str(&format!("{}", status.total));
                } else {
                    // For Firefox output the total as this is the number of failures
                    scores[output_idx].push_str(&format!("{}", status.total - status.passes));
                }
            }
        }
        let mut bugs = BTreeSet::new();
        if let Some(test_meta) = metadata.get(&result.test) {
            for metadata_entry in test_meta.iter() {
                if metadata_entry.product != "firefox"
                    || !metadata_entry
                        .url
                        .starts_with("https://bugzilla.mozilla.org")
                {
                    continue;
                }
                // For now add all bugs irrespective of status or subtest
                if let Ok(bug_url) = Url::parse(&metadata_entry.url) {
                    if let Some((_, bug_id)) = bug_url.query_pairs().find(|(key, _)| key == "id") {
                        bugs.insert(bug_id.into_owned());
                    }
                }
            }
        }
        let mut bugs_col = String::with_capacity(8 * bugs.len());
        for bug in bugs.iter() {
            if !bugs_col.is_empty() {
                bugs_col.push(' ');
            }
            bugs_col.push_str(bug);
        }
        let record = &[&result.test, &scores[0], &scores[1], &scores[2], &bugs_col];
        writer.write_record(record)?;
    }
    Ok(())
}

pub fn interop_columns(focus_areas: &BTreeMap<String, interop::FocusArea>) -> Vec<&str> {
    let mut columns = Vec::with_capacity(focus_areas.len());
    for (name, data) in focus_areas.iter() {
        if data.counts_toward_score {
            columns.push(name.as_ref());
        }
    }
    columns
}

fn browser_score(browser: &str, columns: &[&str], row: &interop::ScoreRow) -> Result<f64> {
    let mut total_score: u64 = 0;
    for column in columns {
        let column = format!("{}-{}", browser, column);
        let score = row
            .get(&column)
            .ok_or_else(|| anyhow!("Failed to get column {}", column))?;
        let value: u64 = score
            .parse::<u64>()
            .map_err(|_| anyhow!("Failed to parse score"))?;
        total_score += value;
    }
    Ok(total_score as f64 / (10 * columns.len()) as f64)
}

pub fn write_browser_interop_scores(
    browsers: &[&str],
    scores: &[interop::ScoreRow],
    interop_2023_data: &interop::YearData,
) -> Result<()> {
    let browser_columns = interop_columns(&interop_2023_data.focus_areas);

    let data_path = Path::new("../docs/interop-2023/scores.csv");
    let out_f = File::create(data_path)?;
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(out_f);

    let mut headers = Vec::with_capacity(browsers.len() + 1);
    headers.push("date");
    headers.extend_from_slice(browsers);
    writer.write_record(headers)?;

    let mut output: Vec<String> = Vec::with_capacity(browsers.len() + 1);

    for row in scores {
        output.resize(0, "".into());
        output.push(
            row.get("date")
                .ok_or_else(|| anyhow!("Failed to read date"))?
                .into(),
        );
        for browser in browsers {
            let score = browser_score(browser, &browser_columns, row)?;
            output.push(format!("{:.2}", score))
        }
        writer.write_record(&output)?;
    }

    Ok(())
}

pub fn run() -> Result<()> {
    let client = network::client();
    let fyi = Wptfyi::new(None);

    let runs = get_run_data(&fyi, &client)?;
    let run_ids = latest_runs(&runs)?
        .iter()
        .map(|x| x.id)
        .collect::<Vec<i64>>();

    let interop_data = get_interop_data(&fyi, &client)?;

    let interop_2023_data = interop_data
        .get("2023")
        .ok_or_else(|| anyhow!("Failed to get Interop-2023 metadata"))?;

    let interop_categories = get_interop_categories(&fyi, &client)?;

    let interop_2023_categories = interop_categories
        .get("2023")
        .ok_or_else(|| anyhow!("Failed to get Interop-2023 categories"))?;
    let categories_by_name = interop_2023_categories.by_name();

    let metadata = get_metadata(&fyi, &client)?;

    for (name, focus_area) in interop_2023_data.focus_areas.iter() {
        write_focus_area(
            &fyi,
            &client,
            name,
            focus_area,
            &run_ids,
            &categories_by_name,
            &metadata,
        )?;
    }

    let scores = get_interop_scores(&fyi, &client, interop::BrowserChannel::Experimental)?;
    write_browser_interop_scores(&["firefox", "chrome", "safari"], &scores, interop_2023_data)?;

    Ok(())
}
