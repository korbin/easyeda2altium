use altium::pcb::Library;

use crate::easyeda::types::Ee3dModel;
use crate::error::Result;

use super::footprint::make_model_entry;

pub fn attach_3d_model(
    library: &mut Library,
    ee_model: &Ee3dModel,
    raw_step: Vec<u8>,
) -> Result<()> {
    if library.models.iter().any(|m| m.id == ee_model.uuid) {
        return Ok(());
    }
    library.models.push(make_model_entry(ee_model, raw_step));
    Ok(())
}
