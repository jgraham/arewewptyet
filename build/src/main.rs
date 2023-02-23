mod failures;
mod interop;
mod latency;
mod network;

use std::process;

fn main() {
    let results = vec![
        failures::update::run(),
        latency::update::run(),
        interop::update::run(),
    ];

    let errors = results
        .iter()
        .filter_map(|x| x.as_ref().err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for err in errors {
            println!("{}", err);
        }
        process::exit(1);
    }
}
