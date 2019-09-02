use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::collections::{HashMap, HashSet};

lazy_static! {
    static ref BACKOUT_RE: Regex = Regex::new(r"Backed out \d+ changeset").unwrap();
    static ref CHANGESET_RE: Regex = Regex::new(r"Backed out changeset ([0-9a-fA-F]+)").unwrap();
    static ref UPDATE_RE: Regex = Regex::new(r".*Update web-platform-tests to ([0-9a-fA-F]+)").unwrap();
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
enum Error {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(io::Error),
    String(String)
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        Error::Reqwest(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Error::Serde(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}


#[derive(Debug, Deserialize)]
struct HgLog {
    entries: Vec<HgLogEntry>
}

#[derive(Debug, Deserialize)]
struct GitHubPr {
    number: u64,
    closed_at: DateTime<Utc>
}

#[derive(Debug, Deserialize)]
struct HgLogEntry {
    node: String,
    desc: String,
    branch: String,
    bookmarks: Vec<String>,
    tags: Vec<String>,
    user: String,
    phase: String,
    parents: Vec<String>,
    pushid: u64,
    date: (f64, i64),
    pushdate: (f64, i64)
}

#[derive(Clone, Debug)]
struct GeckoSyncPoint {
    wpt_rev: String,
    push_date: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct SyncData {
    landings: Vec<SyncPointWptRev>
}

impl SyncData {
    fn new() -> SyncData {
        SyncData {
            landings: Vec::new()
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
        let wpt_rev = UPDATE_RE.captures(&commit.desc)
            .expect("Commit desc must match UPDATE_RE")
            .get(1)
            .expect("Capture group must be present")
            .as_str()
            .to_owned();
        GeckoSyncPoint {
            wpt_rev,
            push_date: commit.pushdate.0 as i64
        }
    }
}

struct LandingData {
    sync_data: SyncData,
    have_shas: HashSet<String>
}

impl LandingData {
    fn new(sync_data: SyncData) -> LandingData {
        let mut have_shas = HashSet::new();
        for sync in sync_data.landings.iter() {
            have_shas.insert(sync.wpt_rev.clone());
        }
        LandingData {
            sync_data,
            have_shas
        }
    }

    fn missing(&self, sync_points: impl Iterator<Item=GeckoSyncPoint>) -> Vec<GeckoSyncPoint> {
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
            wpt_merge_time: pr.closed_at.timestamp(),
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
                    backed_out.entry(short_rev)
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

fn get(client:&reqwest::Client, url:&str, headers: Option<reqwest::header::HeaderMap>) -> Result<String> {
    // TODO - If there's a list then support continuationToken
    println!("{}", url);
    let mut req = client.get(url);
    if let Some(extra_headers) = headers {
        req = req.headers(extra_headers)
    }
    let mut resp = req.send()?;
    resp.error_for_status_ref()?;
    let mut resp_body = match resp.content_length() {
        Some(len) => String::with_capacity(len as usize),
        None => String::new()
    };
    resp.read_to_string(&mut resp_body)?;
    Ok(resp_body)
}

fn get_sync_commits(client:&reqwest::Client) -> Result<String> {
   Ok(get(client,
           "https://hg.mozilla.org/integration/mozilla-inbound/json-log/tip/testing/web-platform/meta/mozilla-sync",
           None)?)
}

fn extract_sync_points(sync_commits: HgLog) -> impl Iterator<Item=GeckoSyncPoint> {
    filter_backouts(sync_commits.entries)
        .into_iter()
        .filter(filter_update)
        .map(|x| x.into())
}


fn get_pr_for_rev(client:&reqwest::Client, wpt_rev: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/web-platform-tests/wpt/commits/{}/pulls", wpt_rev);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/vnd.github.groot-preview+json".parse().unwrap());
    get(client, &url, Some(headers))
}

fn load_sync_data(path: &Path) -> Result<SyncData> {
    if let Ok(f) = File::open(path) {
        Ok(serde_json::from_reader(f)?)
    } else {
        Ok(SyncData::new())
    }
}

fn run() -> Result<()> {
    let client = reqwest::Client::new();

    let sync_commits: HgLog = serde_json::from_str(&get_sync_commits(&client)?)?;
    let sync_points = extract_sync_points(sync_commits);

    let data_path = Path::new("../docs/landings.json");
    let sync_data = load_sync_data(data_path)?;
    let mut landings = LandingData::new(sync_data);
    let missing = landings.missing(sync_points);
    println!("Found {} missing sync points", missing.len());
    for sync_point in missing.into_iter().rev() {
        let pr_data = get_pr_for_rev(&client, &sync_point.wpt_rev)?;
        println!("{}", pr_data);
        let mut prs: Vec<GitHubPr> = serde_json::from_str(&pr_data)?;
        if prs.len() == 0 {
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

fn main() {
    match run() {
        Ok(()) => {},
        Err(e) => {
            println!("{:?}", e);
            process::exit(1);
        }
    }
}
