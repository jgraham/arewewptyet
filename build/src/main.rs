mod failures;
mod interop;
mod latency;
mod network;

use log::error;
use std::process;

fn main() {
    let mut log_builder = env_logger::Builder::new();
    log_builder
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let results = [failures::run(), latency::run(), interop::run()];

    let errors = results
        .iter()
        .filter_map(|x| x.as_ref().err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for err in errors {
            error!("{:?}", err);
        }
        process::exit(1);
    }
}
