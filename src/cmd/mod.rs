mod image;
mod index;
#[cfg(feature = "rocksdb")]
pub mod rocks;
mod server;
mod show;

use crate::config::Opts;
use anyhow::Result;
pub use image::*;
pub use index::*;
pub use server::*;
pub use show::*;

pub trait SubCommandExtend {
    fn run(&self, opts: &Opts) -> Result<()>;
}
