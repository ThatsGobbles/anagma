//! This is intended to be the main public API of the library.

pub mod target;
pub mod plexer;
pub mod processor;
pub mod stream;
pub mod block;
pub mod value;
pub mod structure;

use crate::metadata::processor::Error as ProcessorError;

#[derive(Debug)]
pub enum Error {
    CannotProcessMetadata(ProcessorError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::CannotProcessMetadata(ref err) => write!(f, "cannot process metadata: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::CannotProcessMetadata(ref err) => Some(err),
        }
    }
}

// pub struct Metadata;

// impl Metadata {
//     pub fn get_metadata<'k, P: AsRef<Path>>(item_path: P) -> Result<Block<'k>, Error> {
//         // Use a default configuration and no aggregations.
//         let config = Config::default();

//         Self::get_metadata_with_config(item_path, &config)
//     }

//     pub fn get_metadata_with_config<P: AsRef<Path>>(item_path: P, config: &Config) -> Result<Block, Error> {
//         let mb = Processor::process_item_file(
//             item_path,
//             config.serialize_format,
//             &config.selection,
//             config.sort_order,
//         ).map_err(Error::CannotProcessMetadata)?;

//         Ok(mb)
//     }
// }
