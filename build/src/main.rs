mod error;
mod latency;
mod failures;
mod network;

use std::process;

fn main() {
    let failures_update = failures::update::run();
    let latency_update = latency::update::run();
    let exit_code = match (failures_update, latency_update) {
        (Ok(()), Ok(())) => 0,
        (x, y) => {
            if let Err(e) = x {
                println!("Failures update failed:\n{:?}", e);
            };
            if let Err(e) = y {
                println!("Latency update failed:\n{:?}", e);
            };
            1
        }
    };
    process::exit(exit_code);
}
