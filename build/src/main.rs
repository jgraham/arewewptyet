mod failures;
mod interop;
mod latency;
mod network;

use std::process;

fn main() {
    let year = 2025;
    let results = [failures::run(), latency::run(), interop::run(year)];

    let errors = results
        .iter()
        .filter_map(|x| x.as_ref().err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for err in errors {
            eprintln!("{:?}", err);
        }
        process::exit(1);
    }
}
