#![allow(missing_docs)]
mod process_text_file;
pub use process_text_file::*;

mod walk_directory;
pub use walk_directory::*;

mod utils;
pub use utils::*;

mod replace;
pub use replace::*;

pub mod templates;
