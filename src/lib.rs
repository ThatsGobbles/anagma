pub mod config;
pub mod metadata;
pub mod sources;
pub mod types;
mod util;

#[cfg(test)] mod test_util;

use std::path::Path;

use crate::config::Config;
use crate::metadata::processor::Processor;
use crate::types::Block;

pub use crate::util::FileWalker;

pub fn get<P: AsRef<Path>>(path: &P) -> Block {
    let config = Config::default();
    get_with_config(path, &config)
}

pub fn get_with_config<P: AsRef<Path>>(path: &P, config: &Config) -> Block {
    Processor::process_item_file(
        path.as_ref(),
        &config.sourcer,
        &config.selection,
        &config.sorter,
    ).unwrap()
}
