use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::serde::iso8601;
use time::OffsetDateTime;

lazy_static! {
    static ref BACKOUT_RE: Regex = Regex::new(r"Backed out \d+ changeset").unwrap();
    static ref CHANGESET_RE: Regex = Regex::new(r"Backed out changeset ([0-9a-fA-F]+)").unwrap();
    static ref UPDATE_RE: Regex =
        Regex::new(r".*Update web-platform-tests to ([0-9a-fA-F]+)").unwrap();
}

#[derive(Debug, Deserialize)]
pub struct HgLog {
    entries: Vec<HgLogEntry>,
}

#[derive(Debug, Deserialize)]
struct GitHubPr {
    number: u64,
    #[serde(with = "iso8601")]
    closed_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct HgLogEntry {
    node: String,
    desc: String,
    pushdate: (f64, i64),
}

#[derive(Clone, Debug)]
pub struct GeckoSyncPoint {
    wpt_rev: String,
    push_date: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct SyncData {
    landings: Vec<SyncPointWptRev>,
}

impl SyncData {
    fn new() -> SyncData {
        SyncData {
            landings: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SyncPointWptRev {
    wpt_rev: String,
    wpt_pr: u64,
    wpt_merge_time: i64,
    gecko_push_time: i64,
}

impl From<HgLogEntry> for GeckoSyncPoint {
    fn from(commit: HgLogEntry) -> GeckoSyncPoint {
        // This will panic if the commit doesn't have the correct metadata
        let wpt_rev = UPDATE_RE
            .captures(&commit.desc)
            .expect("Commit desc must match UPDATE_RE")
            .get(1)
            .expect("Capture group must be present")
            .as_str()
            .to_owned();
        GeckoSyncPoint {
            wpt_rev,
            push_date: commit.pushdate.0 as i64,
        }
    }
}

pub struct LandingData {
    sync_data: SyncData,
    have_shas: HashSet<String>,
}

impl LandingData {
    fn new(sync_data: SyncData) -> LandingData {
        let mut have_shas = HashSet::new();
        for sync in sync_data.landings.iter() {
            have_shas.insert(sync.wpt_rev.clone());
        }
        LandingData {
            sync_data,
            have_shas,
        }
    }

    fn missing(&self, sync_points: impl Iterator<Item = GeckoSyncPoint>) -> Vec<GeckoSyncPoint> {
        let mut rv = Vec::new();
        for sync_point in sync_points {
            if !self.have_shas.contains(&sync_point.wpt_rev) {
                rv.push(sync_point.clone());
            }
        }
        rv
    }

    fn insert(&mut self, sync_point: GeckoSyncPoint, pr: GitHubPr) {
        let data = SyncPointWptRev {
            wpt_rev: sync_point.wpt_rev,
            wpt_pr: pr.number,
            wpt_merge_time: pr.closed_at.unix_timestamp(),
            gecko_push_time: sync_point.push_date,
        };
        self.have_shas.insert(data.wpt_rev.clone());
        self.sync_data.landings.push(data);
    }
}

fn filter_backouts(commits: Vec<HgLogEntry>) -> Vec<HgLogEntry> {
    let mut backed_out: HashMap<String, HashSet<String>> = HashMap::new();
    let mut filtered_commits = Vec::with_capacity(commits.len());

    for commit in commits.into_iter() {
        if BACKOUT_RE.is_match(&commit.desc) {
            for line in commit.desc.lines() {
                let changeset = CHANGESET_RE.captures(line);
                if let Some(captures) = changeset {
                    let changeset_rev = captures
                        .get(1)
                        .expect("Couldn't get changeset from rev")
                        .as_str()
                        .to_owned();
                    let short_rev = changeset_rev[0..12].to_owned();
                    backed_out
                        .entry(short_rev)
                        .and_modify(|v| {
                            v.insert(changeset_rev.clone());
                        })
                        .or_insert_with(|| {
                            let mut entry = HashSet::new();
                            entry.insert(changeset_rev);
                            entry
                        });
                }
            }
        } else {
            let short_rev = &commit.node[0..12];
            let is_backed_out = if backed_out.contains_key(short_rev) {
                let full_revs = backed_out.get_mut(short_rev).unwrap();
                full_revs.remove(&commit.node)
            } else {
                false
            };
            if !is_backed_out {
                filtered_commits.push(commit);
            }
        }
    }
    filtered_commits
}

fn filter_update(commit: &HgLogEntry) -> bool {
    UPDATE_RE.is_match(&commit.desc)
}

pub fn extract_sync_points(sync_commits: &str) -> Result<impl Iterator<Item = GeckoSyncPoint>> {
    let sync_commits: HgLog = serde_json::from_str(sync_commits)?;
    Ok(filter_backouts(sync_commits.entries)
        .into_iter()
        .filter(filter_update)
        .map(|x| x.into()))
}

pub mod update {
    use super::*;
    use serde_json;
    use std::fs::File;
    use std::path::Path;

    use crate::network::{self, get};

    fn get_sync_commits(client: &reqwest::blocking::Client) -> Result<String> {
        get(client,
            "https://hg.mozilla.org/integration/autoland/json-log/tip/testing/web-platform/meta/mozilla-sync",
            None)
    }

    fn get_pr_for_rev(client: &reqwest::blocking::Client, wpt_rev: &str) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/web-platform-tests/wpt/commits/{}/pulls",
            wpt_rev
        );
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            "application/vnd.github.groot-preview+json".parse().unwrap(),
        );
        get(client, &url, Some(headers))
    }

    fn load_sync_data(path: &Path) -> Result<SyncData> {
        if let Ok(f) = File::open(path) {
            Ok(serde_json::from_reader(f)?)
        } else {
            Ok(SyncData::new())
        }
    }

    pub fn run() -> Result<()> {
        let client = network::client();

        let sync_points_data = get_sync_commits(&client)?;
        let sync_points = extract_sync_points(&sync_points_data)?;

        let data_path = Path::new("../docs/landings.json");
        let sync_data = load_sync_data(data_path)?;
        let mut landings = LandingData::new(sync_data);
        let missing = landings.missing(sync_points);
        println!("Found {} missing sync points", missing.len());
        for sync_point in missing.into_iter().rev() {
            let pr_data = get_pr_for_rev(&client, &sync_point.wpt_rev)?;
            println!("{}", pr_data);
            let mut prs: Vec<GitHubPr> = serde_json::from_str(&pr_data)?;
            if prs.is_empty() {
                println!("No PR found for commit {}", &sync_point.wpt_rev);
                continue;
            } else if prs.len() > 1 {
                println!("Multiple PRs found for commit {}", &sync_point.wpt_rev);
            }
            let pr = prs.remove(0);
            landings.insert(sync_point, pr);
        }

        let out_f = File::create(data_path)?;
        serde_json::to_writer(out_f, &landings.sync_data)?;
        Ok(())
    }
}
