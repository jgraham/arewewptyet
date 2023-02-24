mod failures;
mod interop;
mod latency;
mod network;

use std::process;

fn main() {
    let results = vec![failures::run(), latency::run(), interop::run()];

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
