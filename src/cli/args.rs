use clap::Parser;

use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(
    name = "easyeda2altium",
    about = "Convert LCSC/EasyEDA components into Altium .SchLib / .PcbLib"
)]
pub struct Args {
    /// One or more LCSC component IDs (e.g. C2040 C25744).
    #[arg(long = "lcsc_id", num_args = 1.., required = true)]
    pub lcsc_id: Vec<String>,

    /// Emit symbol library (.SchLib).
    #[arg(long)]
    pub symbol: bool,

    /// Emit footprint library (.PcbLib).
    #[arg(long)]
    pub footprint: bool,

    /// Emit 3D model (embedded STEP in .PcbLib's Library/Models).
    #[arg(long = "3d")]
    pub three_d: bool,

    /// Emit symbol + footprint + 3D model.
    #[arg(long)]
    pub full: bool,

    /// Output basename. `--output /tmp/lib` produces /tmp/lib.SchLib and
    /// /tmp/lib.PcbLib. Defaults to ~/Documents/easyeda2altium/easyeda2altium.
    #[arg(long)]
    pub output: Option<String>,

    /// Replace existing output files.
    #[arg(long)]
    pub overwrite: bool,

    /// Cache API responses in ./.easyeda_cache/ to skip network on repeats.
    #[arg(long = "use-cache")]
    pub use_cache: bool,

    /// Add custom symbol parameters: --custom-field 'Manufacturer:TI' 'Tol:1%'.
    #[arg(long = "custom-field", num_args = 0..)]
    pub custom_field: Vec<String>,

    /// Strip CJK ideographs (and any wrapping parens/whitespace) from
    /// human-visible strings.
    #[arg(long = "strip-chinese")]
    pub strip_chinese: bool,

    /// Font name to use for all text in the emitted .SchLib and .PcbLib.
    #[arg(long = "font")]
    pub font: Option<String>,
}

impl Args {
    pub fn validate(&self) -> Result<()> {
        for id in &self.lcsc_id {
            if !id.starts_with('C') {
                return Err(Error::InvalidArg(format!(
                    "lcsc_id '{}' must start with 'C'",
                    id
                )));
            }
        }
        Ok(())
    }

    pub fn parsed_custom_fields(&self) -> Result<Vec<(String, String)>> {
        let mut out = Vec::with_capacity(self.custom_field.len());
        for raw in &self.custom_field {
            let (k, v) = raw
                .split_once(':')
                .ok_or_else(|| Error::InvalidArg(format!("custom-field '{}' missing ':'", raw)))?;
            let k = k.trim().to_string();
            let v = v.trim().to_string();
            if k.is_empty() {
                return Err(Error::InvalidArg(format!(
                    "custom-field '{}' has empty key",
                    raw
                )));
            }
            out.push((k, v));
        }
        Ok(out)
    }
}
