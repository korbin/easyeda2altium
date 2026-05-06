//! Convert LCSC/EasyEDA components to Altium SchLib / PcbLib.

pub mod cli;
pub mod convert;
pub mod easyeda;
pub mod error;

pub use error::{Error, Result};
