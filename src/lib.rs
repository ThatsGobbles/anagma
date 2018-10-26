#![feature(generators)]
#![feature(generator_trait)]
#![feature(non_exhaustive)]

#[macro_use] extern crate failure;
#[macro_use] extern crate maplit;
extern crate yaml_rust;
extern crate serde;
extern crate serde_yaml;
#[macro_use] extern crate serde_derive;
extern crate globset;
extern crate itertools;
#[macro_use] extern crate log;

#[cfg(test)] extern crate tempdir;

mod library;
mod metadata;
mod util;
mod error;

#[cfg(test)] mod test_util;
