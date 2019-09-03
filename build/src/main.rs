mod error;
mod latency;
mod failures;
mod network;

use std::process;

fn main() {
    match failures::update::run() {
        Ok(()) => {},
        Err(e) => {
            println!("{:?}", e);
            process::exit(1);
        }
    }
}
