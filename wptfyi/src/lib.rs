pub mod error;
pub mod interop;
pub mod metadata;
pub mod result;
pub mod run;
pub mod search;

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

    pub fn metadata(&self) -> metadata::Metadata {
        metadata::Metadata::new(self.host.clone())
    }

    pub fn interop_scores(&self, browser_channel: interop::BrowserChannel) -> interop::ScoreData {
        interop::ScoreData::new(browser_channel)
    }
}
