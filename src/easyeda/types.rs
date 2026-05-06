use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EePinElectrical {
    Unspecified = 0,
    Input = 1,
    Output = 2,
    Bidirectional = 3,
    Power = 4,
}

impl EePinElectrical {
    pub fn from_int(v: i64) -> Self {
        match v {
            1 => Self::Input,
            2 => Self::Output,
            3 => Self::Bidirectional,
            4 => Self::Power,
            _ => Self::Unspecified,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EeBbox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolInfo {
    pub name: String,
    pub prefix: String,
    pub package: String,
    pub manufacturer: String,
    pub mpn: String,
    pub datasheet: String,
    pub lcsc_id: String,
    pub keywords: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct EeSymbolPin {
    pub electrical: EePinElectrical,
    pub designator: String, // pin number, e.g. "1", "VCC"
    pub pos_x: f64,         // pin attachment point (where wire connects)
    pub pos_y: f64,
    pub rotation_deg: i32, // 0 / 90 / 180 / 270
    pub name: String,
    pub pin_length: f64,
    pub is_displayed: bool,
    pub is_inverted: bool,
    pub is_clock: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolRectangle {
    pub pos_x: f64,
    pub pos_y: f64,
    pub width: f64,
    pub height: f64,
    pub fill: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolCircle {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub fill: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolEllipse {
    pub center_x: f64,
    pub center_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub fill: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolPolyline {
    pub points: Vec<(f64, f64)>,
    pub fill: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolArc {
    /// SVG path `M sx sy A rx ry rot large_arc sweep ex ey`
    pub raw_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbolText {
    pub pos_x: f64,
    pub pos_y: f64,
    pub rotation_deg: f64,
    pub text: String,
    pub font_size_mm: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EeSymbol {
    pub info: EeSymbolInfo,
    pub bbox: EeBbox,
    pub pins: Vec<EeSymbolPin>,
    pub rectangles: Vec<EeSymbolRectangle>,
    pub circles: Vec<EeSymbolCircle>,
    pub ellipses: Vec<EeSymbolEllipse>,
    pub polylines: Vec<EeSymbolPolyline>,
    pub arcs: Vec<EeSymbolArc>,
    pub texts: Vec<EeSymbolText>,
    pub sub_symbols: Vec<EeSymbol>,
}

// ---------- Footprint ----------

#[derive(Debug, Clone, Default)]
pub struct EeFootprintInfo {
    pub name: String,
    pub fp_type: String, // "smd" or "tht"
    pub model_3d_uuid: String,
    pub manufacturer: String,
    pub mpn: String,
    pub lcsc_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintPad {
    pub shape: String, // "ELLIPSE", "RECT", "OVAL", "POLYGON"
    pub center_x: f64,
    pub center_y: f64,
    pub width: f64,
    pub height: f64,
    pub layer_id: i32,
    pub number: String,
    pub hole_radius: f64, // 0 for SMD
    pub points: String,   // polygon-shape custom outline (raw EE pixels)
    pub rotation_deg: f64,
    pub hole_length: f64, // slot
    pub is_plated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintTrack {
    pub stroke_width: f64,
    pub layer_id: i32,
    pub points: String,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintHole {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintVia {
    pub center_x: f64,
    pub center_y: f64,
    pub diameter: f64,
    pub drill_radius: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintCircle {
    pub cx: f64,
    pub cy: f64,
    pub radius: f64,
    pub stroke_width: f64,
    pub layer_id: i32,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintArc {
    pub stroke_width: f64,
    pub layer_id: i32,
    pub raw_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub layer_id: i32,
    pub stroke_width: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintText {
    pub kind: String, // "P" prefix, "N" name
    pub center_x: f64,
    pub center_y: f64,
    pub stroke_width: f64,
    pub rotation_deg: f64,
    pub layer_id: i32,
    pub font_size: f64,
    pub text: String,
    pub is_displayed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprintSolidRegion {
    pub layer_id: i32,
    pub raw_path: String,
    pub region_type: String, // "solid", "cutout", "npth"
}

#[derive(Debug, Clone, Default)]
pub struct EeFootprint {
    pub info: EeFootprintInfo,
    pub bbox_x: f64,
    pub bbox_y: f64,
    pub pads: Vec<EeFootprintPad>,
    pub tracks: Vec<EeFootprintTrack>,
    pub holes: Vec<EeFootprintHole>,
    pub vias: Vec<EeFootprintVia>,
    pub circles: Vec<EeFootprintCircle>,
    pub arcs: Vec<EeFootprintArc>,
    pub rectangles: Vec<EeFootprintRect>,
    pub texts: Vec<EeFootprintText>,
    pub solid_regions: Vec<EeFootprintSolidRegion>,
    pub model_3d: Option<Ee3dModel>,
}

#[derive(Debug, Clone, Default)]
pub struct Ee3dModel {
    pub uuid: String,
    pub name: String,
    pub translation_x_mm: f64,
    pub translation_y_mm: f64,
    pub translation_z_mm: f64,
    pub rotation_x_deg: f64,
    pub rotation_y_deg: f64,
    pub rotation_z_deg: f64,
    pub raw_step: Option<Vec<u8>>,
}

// ---------- safe_* helpers ----------

pub fn safe_float(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

pub fn safe_int(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n.as_i64().unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64),
        Value::String(s) => s
            .trim()
            .parse::<f64>()
            .map(|x| x as i64)
            .unwrap_or(0),
        Value::Bool(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn safe_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::String(s) => {
            let lc = s.to_ascii_lowercase();
            matches!(lc.as_str(), "true" | "1" | "yes" | "on" | "show" | "y" | "t")
        }
        Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

pub fn safe_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn safe_float_handles_strings() {
        assert_eq!(safe_float(&json!("3.14")), 3.14);
        assert_eq!(safe_float(&json!(2)), 2.0);
        assert_eq!(safe_float(&json!("not a number")), 0.0);
    }

    #[test]
    fn safe_bool_strings() {
        assert!(safe_bool(&json!("true")));
        assert!(safe_bool(&json!("Y")));
        assert!(!safe_bool(&json!("false")));
        assert!(!safe_bool(&json!("")));
    }
}
