//! Parse `dataStr.shape: Vec<String>` from the EasyEDA API into typed
//! `EeSymbol` / `EeFootprint`.
//!
//! Format reference: `easyeda2kicad/easyeda/easyeda_importer.py`. Each shape
//! is a `~`-separated string starting with a designator. Pins use `^^` for
//! sub-segments.

use serde_json::Value;

use super::types::*;
use crate::error::{Error, Result};

// ---------- helpers ----------

fn parse_f(s: Option<&str>) -> f64 {
    s.and_then(|x| x.trim().parse::<f64>().ok()).unwrap_or(0.0)
}

fn parse_i(s: Option<&str>) -> i32 {
    s.and_then(|x| x.trim().parse::<f64>().ok())
        .map(|x| x as i32)
        .unwrap_or(0)
}

fn parse_b(s: Option<&str>) -> bool {
    match s {
        Some(v) => {
            let lc = v.trim().to_ascii_lowercase();
            matches!(
                lc.as_str(),
                "true" | "1" | "yes" | "on" | "show" | "y" | "t"
            )
        }
        None => false,
    }
}

fn at<'a>(parts: &'a [&str], idx: usize) -> Option<&'a str> {
    parts.get(idx).copied()
}

// ---------- symbol top-level ----------

pub fn parse_symbol(api_result: &Value) -> Result<EeSymbol> {
    let datastr = api_result
        .get("dataStr")
        .ok_or_else(|| Error::Parse("missing dataStr".into()))?;
    let head = datastr
        .get("head")
        .ok_or_else(|| Error::Parse("missing dataStr.head".into()))?;
    let bbox = datastr.get("BBox").cloned().unwrap_or(Value::Null);
    let c_para = head.get("c_para").cloned().unwrap_or(Value::Null);
    let lcsc = api_result.get("lcsc").cloned().unwrap_or(Value::Null);
    let description = safe_string(api_result.get("description").unwrap_or(&Value::Null));
    let mut sym = build_symbol_unit(datastr, &c_para, &lcsc, &bbox, head, &description)?;

    // Sub-symbols for multi-unit components.
    if let Some(arr) = api_result.get("subparts").and_then(|v| v.as_array()) {
        for sp in arr {
            let sp_datastr = sp
                .get("dataStr")
                .ok_or_else(|| Error::Parse("subpart missing dataStr".into()))?;
            let sp_head = sp_datastr
                .get("head")
                .ok_or_else(|| Error::Parse("subpart missing dataStr.head".into()))?;
            let sp_bbox = sp_datastr.get("BBox").cloned().unwrap_or(Value::Null);
            let sp_cpara = sp_head.get("c_para").cloned().unwrap_or(Value::Null);
            let sub =
                build_symbol_unit(sp_datastr, &sp_cpara, &lcsc, &sp_bbox, sp_head, &description)?;
            sym.sub_symbols.push(sub);
        }
    }
    Ok(sym)
}

fn build_symbol_unit(
    datastr: &Value,
    c_para: &Value,
    lcsc: &Value,
    bbox: &Value,
    head: &Value,
    description: &str,
) -> Result<EeSymbol> {
    let info = EeSymbolInfo {
        name: sanitize_component_name(&safe_string(c_para.get("name").unwrap_or(&Value::Null))),
        prefix: safe_string(c_para.get("pre").unwrap_or(&Value::Null)),
        package: safe_string(c_para.get("package").unwrap_or(&Value::Null)),
        manufacturer: pick_str(c_para, &["Manufacturer", "BOM_Manufacturer"]),
        mpn: pick_str(c_para, &["Manufacturer Part", "BOM_Manufacturer Part"]),
        datasheet: {
            let url = safe_string(lcsc.get("url").unwrap_or(&Value::Null));
            let lcsc_id = safe_string(lcsc.get("number").unwrap_or(&Value::Null));
            if !url.is_empty() {
                url
            } else if !lcsc_id.is_empty() {
                format!("https://www.lcsc.com/datasheet/{}.pdf", lcsc_id)
            } else {
                String::new()
            }
        },
        lcsc_id: safe_string(lcsc.get("number").unwrap_or(&Value::Null)),
        keywords: tags_to_string(datastr.get("tags")),
        description: description.to_string(),
    };

    let bbox_x = safe_float(bbox.get("x").unwrap_or(&Value::Null));
    let bbox_y = safe_float(bbox.get("y").unwrap_or(&Value::Null));
    let bbox_w = safe_float(bbox.get("width").unwrap_or(&Value::Null));
    let bbox_h = safe_float(bbox.get("height").unwrap_or(&Value::Null));
    let (origin_x, origin_y) = if bbox_w > 0.0 || bbox_h > 0.0 {
        (bbox_x + bbox_w / 2.0, bbox_y + bbox_h / 2.0)
    } else {
        (
            safe_float(head.get("x").unwrap_or(&Value::Null)),
            safe_float(head.get("y").unwrap_or(&Value::Null)),
        )
    };

    let mut sym = EeSymbol {
        info,
        bbox: EeBbox {
            x: origin_x,
            y: origin_y,
            width: bbox_w,
            height: bbox_h,
        },
        ..Default::default()
    };

    if let Some(shapes) = datastr.get("shape").and_then(|v| v.as_array()) {
        for shape in shapes {
            if let Some(s) = shape.as_str() {
                parse_symbol_shape(s, &mut sym);
            }
        }
    }
    Ok(sym)
}

fn pick_str(c_para: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(v) = c_para.get(*k) {
            let s = safe_string(v);
            if !s.is_empty() {
                return s;
            }
        }
    }
    String::new()
}

fn tags_to_string(tags: Option<&Value>) -> String {
    let Some(arr) = tags.and_then(|v| v.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_component_name(name: &str) -> String {
    let s = name;
    let mut end = s.len();
    for ch in ['(', '['] {
        if let Some(idx) = s.find(ch) {
            end = end.min(idx);
        }
    }
    s[..end].trim().to_string()
}

// ---------- symbol shape dispatch ----------

pub fn parse_symbol_shape(shape: &str, sym: &mut EeSymbol) {
    let designator = match shape.split('~').next() {
        Some(d) => d,
        None => return,
    };
    match designator {
        "P" => parse_pin(shape, sym),
        "R" => parse_rectangle(shape, sym),
        "C" => parse_circle(shape, sym),
        "E" => parse_ellipse(shape, sym),
        "PL" => parse_polyline(shape, sym, false),
        "PG" => parse_polyline(shape, sym, true),
        "A" => parse_arc(shape, sym),
        "T" => parse_text(shape, sym),
        "PT" => {}
        _ => {}
    }
}

fn parse_pin(shape: &str, sym: &mut EeSymbol) {
    // Format: P~settings^^dot^^path^^name^^num^^dot_bis^^clock
    //   settings: visibility~type~spice_pin_number~pos_x~pos_y~rotation~id~is_locked
    let segments: Vec<&str> = shape.split("^^").collect();
    if segments.is_empty() {
        return;
    }
    let settings: Vec<&str> = segments[0].split('~').collect();
    if settings.len() < 7 {
        return;
    }
    let is_displayed = parse_b(at(&settings, 1));
    let pin_type = parse_i(at(&settings, 2));
    let pos_x = parse_f(at(&settings, 4));
    let pos_y = parse_f(at(&settings, 5));
    let rotation = parse_i(at(&settings, 6));

    // Pin number lives in segment 4[4] (per Python's importer)
    let designator = if segments.len() > 4 {
        let nseg: Vec<&str> = segments[4].split('~').collect();
        nseg.get(4).copied().unwrap_or("").to_string()
    } else {
        String::new()
    };

    let pin_length = if segments.len() > 2 {
        let path_seg: Vec<&str> = segments[2].split('~').collect();
        let path = path_seg.first().copied().unwrap_or("");
        derive_pin_length(path)
    } else {
        0.0
    };

    // Name (segment 3 index 4)
    let name = if segments.len() > 3 {
        let nseg: Vec<&str> = segments[3].split('~').collect();
        nseg.get(4).copied().unwrap_or("").to_string()
    } else {
        String::new()
    };

    // dot (segment 5 index 0)
    let is_inverted = if segments.len() > 5 {
        let dseg: Vec<&str> = segments[5].split('~').collect();
        dseg.first().copied().map(|s| s != "0" && !s.is_empty()).unwrap_or(false)
    } else {
        false
    };
    // clock (segment 6 index 0)
    let is_clock = if segments.len() > 6 {
        let cseg: Vec<&str> = segments[6].split('~').collect();
        cseg.first().copied().map(|s| s != "0" && !s.is_empty()).unwrap_or(false)
    } else {
        false
    };

    sym.pins.push(EeSymbolPin {
        electrical: EePinElectrical::from_int(pin_type as i64),
        designator,
        pos_x,
        pos_y,
        rotation_deg: rotation,
        name,
        pin_length,
        is_displayed,
        is_inverted,
        is_clock,
    });
}

/// Pin path is one of `M x y h <len>` or `M x y v <len>`. Extract |len|.
fn derive_pin_length(path: &str) -> f64 {
    // Find the last `h` or `v` and parse the number after it.
    let path = path.trim();
    let mut tokens = path.split_whitespace().peekable();
    let mut last_dir = None;
    let mut last_value = 0.0f64;
    while let Some(t) = tokens.next() {
        if t == "h" || t == "H" || t == "v" || t == "V" {
            last_dir = Some(t);
            if let Some(num) = tokens.peek() {
                last_value = num.parse::<f64>().unwrap_or(0.0).abs();
            }
        }
    }
    let _ = last_dir;
    last_value
}

fn parse_rectangle(shape: &str, sym: &mut EeSymbol) {
    // R~x~y~rx~ry~width~height~stroke~stroke_width~stroke_style~fill~id~locked
    // OR R~x~y~~~width~height~... (no rounded corners)
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 7 {
        return;
    }
    let pos_x = parse_f(at(&parts, 1));
    let pos_y = parse_f(at(&parts, 2));
    let (width, height, fill_idx) = if at(&parts, 3) == Some("") && at(&parts, 4) == Some("") {
        (parse_f(at(&parts, 5)), parse_f(at(&parts, 6)), 10usize)
    } else {
        (parse_f(at(&parts, 5)), parse_f(at(&parts, 6)), 10usize)
    };
    let fill_color = at(&parts, fill_idx).unwrap_or("none");
    let fill = !matches!(fill_color, "" | "none" | "None");
    sym.rectangles.push(EeSymbolRectangle {
        pos_x,
        pos_y,
        width,
        height,
        fill,
    });
}

fn parse_circle(shape: &str, sym: &mut EeSymbol) {
    // C~cx~cy~radius~stroke~stroke_width~stroke_style~fill~id~locked
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 4 {
        return;
    }
    let fill_color = at(&parts, 7).unwrap_or("none");
    sym.circles.push(EeSymbolCircle {
        center_x: parse_f(at(&parts, 1)),
        center_y: parse_f(at(&parts, 2)),
        radius: parse_f(at(&parts, 3)),
        fill: !matches!(fill_color, "" | "none" | "None"),
    });
}

fn parse_ellipse(shape: &str, sym: &mut EeSymbol) {
    // E~cx~cy~rx~ry~stroke~stroke_width~stroke_style~fill~id~locked
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 5 {
        return;
    }
    let fill_color = at(&parts, 8).unwrap_or("none");
    sym.ellipses.push(EeSymbolEllipse {
        center_x: parse_f(at(&parts, 1)),
        center_y: parse_f(at(&parts, 2)),
        radius_x: parse_f(at(&parts, 3)),
        radius_y: parse_f(at(&parts, 4)),
        fill: !matches!(fill_color, "" | "none" | "None"),
    });
}

fn parse_polyline(shape: &str, sym: &mut EeSymbol, closed: bool) {
    // PL/PG~points~stroke~stroke_width~stroke_style~fill~id~locked
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 2 {
        return;
    }
    let points = parse_point_pairs(parts[1]);
    let fill_color = at(&parts, 5).unwrap_or("none");
    let fill = closed || !matches!(fill_color, "" | "none" | "None");
    sym.polylines.push(EeSymbolPolyline { points, fill });
}

fn parse_point_pairs(s: &str) -> Vec<(f64, f64)> {
    let toks: Vec<&str> = s.split_whitespace().collect();
    let mut out = Vec::with_capacity(toks.len() / 2);
    let mut i = 0;
    while i + 1 < toks.len() {
        let x = toks[i].parse::<f64>().unwrap_or(0.0);
        let y = toks[i + 1].parse::<f64>().unwrap_or(0.0);
        out.push((x, y));
        i += 2;
    }
    out
}

fn parse_arc(shape: &str, sym: &mut EeSymbol) {
    // A~path~helper_dots~stroke~stroke_width~stroke_style~fill~id~locked
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 2 {
        return;
    }
    sym.arcs.push(EeSymbolArc {
        raw_path: parts[1].to_string(),
    });
}

fn parse_text(shape: &str, sym: &mut EeSymbol) {
    // T~type~x~y~rotation~color~font~font_size~stroke~baseline~anchor~role~text~display~...
    let parts: Vec<&str> = shape.split('~').collect();
    if parts.len() < 13 || parts[12].is_empty() {
        return;
    }
    let font_size_str = parts.get(7).copied().unwrap_or("7pt");
    let font_size_mm = font_size_str
        .trim_end_matches("pt")
        .parse::<f64>()
        .map(|v| v * 0.3528)
        .unwrap_or(1.27);

    sym.texts.push(EeSymbolText {
        pos_x: parse_f(at(&parts, 2)),
        pos_y: parse_f(at(&parts, 3)),
        rotation_deg: parse_f(at(&parts, 4)),
        text: parts[12].to_string(),
        font_size_mm,
    });
}

// ---------- footprint ----------

pub fn parse_footprint(api_result: &Value) -> Result<EeFootprint> {
    let pkg = api_result
        .get("packageDetail")
        .ok_or_else(|| Error::Parse("missing packageDetail".into()))?;
    let datastr = pkg
        .get("dataStr")
        .ok_or_else(|| Error::Parse("missing packageDetail.dataStr".into()))?;
    let head = datastr
        .get("head")
        .ok_or_else(|| Error::Parse("missing packageDetail.dataStr.head".into()))?;
    let c_para = head.get("c_para").cloned().unwrap_or(Value::Null);

    let assembly = api_result
        .get("customData")
        .and_then(|v| v.get("jlcPara"))
        .and_then(|v| v.get("assemblyProcess"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let title = pkg
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let is_smd = if !assembly.is_empty() {
        assembly.eq_ignore_ascii_case("SMT")
    } else {
        api_result.get("SMT").map(|v| v.as_bool().unwrap_or(false)).unwrap_or(false)
            && !title.contains("-TH_")
    };

    let info = EeFootprintInfo {
        name: safe_string(c_para.get("package").unwrap_or(&Value::Null)),
        fp_type: if is_smd { "smd".into() } else { "tht".into() },
        model_3d_uuid: safe_string(c_para.get("3DModel").unwrap_or(&Value::Null)),
        manufacturer: pick_str(&c_para, &["Manufacturer", "BOM_Manufacturer"]),
        mpn: pick_str(&c_para, &["Manufacturer Part", "BOM_Manufacturer Part"]),
        lcsc_id: safe_string(
            api_result
                .get("lcsc")
                .and_then(|v| v.get("number"))
                .unwrap_or(&Value::Null),
        ),
        description: safe_string(api_result.get("description").unwrap_or(&Value::Null)),
    };

    let bbox_x = safe_float(head.get("x").unwrap_or(&Value::Null));
    let bbox_y = safe_float(head.get("y").unwrap_or(&Value::Null));

    let mut fp = EeFootprint {
        info,
        bbox_x,
        bbox_y,
        ..Default::default()
    };

    // canvas origin override (canvas string positions 16,17)
    let mut canvas_ox = bbox_x;
    let mut canvas_oy = bbox_y;
    if let Some(c) = datastr.get("canvas").and_then(|v| v.as_str()) {
        let c_parts: Vec<&str> = c.split('~').collect();
        if c_parts.len() > 17 {
            canvas_ox = parse_f(c_parts.get(16).copied());
            canvas_oy = parse_f(c_parts.get(17).copied());
        }
    }

    if let Some(shapes) = datastr.get("shape").and_then(|v| v.as_array()) {
        for s in shapes {
            if let Some(text) = s.as_str() {
                parse_footprint_shape(text, &mut fp, canvas_ox, canvas_oy);
            }
        }
    }
    Ok(fp)
}

pub fn parse_footprint_shape(
    shape: &str,
    fp: &mut EeFootprint,
    canvas_ox: f64,
    canvas_oy: f64,
) {
    let parts: Vec<&str> = shape.split('~').collect();
    let designator = match parts.first() {
        Some(d) => d,
        None => return,
    };
    match *designator {
        "PAD" => {
            if parts.len() < 7 {
                return;
            }
            let is_plated = at(&parts, 15).map(|v| v != "N").unwrap_or(true);
            fp.pads.push(EeFootprintPad {
                shape: at(&parts, 1).unwrap_or("").to_string(),
                center_x: parse_f(at(&parts, 2)),
                center_y: parse_f(at(&parts, 3)),
                width: parse_f(at(&parts, 4)),
                height: parse_f(at(&parts, 5)),
                layer_id: parse_i(at(&parts, 6)),
                number: at(&parts, 8).unwrap_or("").to_string(),
                hole_radius: parse_f(at(&parts, 9)),
                points: at(&parts, 10).unwrap_or("").to_string(),
                rotation_deg: parse_f(at(&parts, 11)),
                hole_length: parse_f(at(&parts, 13)),
                is_plated,
            });
        }
        "TRACK" => {
            if parts.len() < 5 {
                return;
            }
            fp.tracks.push(EeFootprintTrack {
                stroke_width: parse_f(at(&parts, 1)),
                layer_id: parse_i(at(&parts, 2)),
                points: at(&parts, 4).unwrap_or("").to_string(),
            });
        }
        "HOLE" => {
            if parts.len() < 4 {
                return;
            }
            fp.holes.push(EeFootprintHole {
                center_x: parse_f(at(&parts, 1)),
                center_y: parse_f(at(&parts, 2)),
                radius: parse_f(at(&parts, 3)),
            });
        }
        "VIA" => {
            if parts.len() < 6 {
                return;
            }
            fp.vias.push(EeFootprintVia {
                center_x: parse_f(at(&parts, 1)),
                center_y: parse_f(at(&parts, 2)),
                diameter: parse_f(at(&parts, 3)),
                drill_radius: parse_f(at(&parts, 5)),
            });
        }
        "CIRCLE" => {
            if parts.len() < 6 {
                return;
            }
            fp.circles.push(EeFootprintCircle {
                cx: parse_f(at(&parts, 1)),
                cy: parse_f(at(&parts, 2)),
                radius: parse_f(at(&parts, 3)),
                stroke_width: parse_f(at(&parts, 4)),
                layer_id: parse_i(at(&parts, 5)),
            });
        }
        "ARC" => {
            if parts.len() < 5 {
                return;
            }
            fp.arcs.push(EeFootprintArc {
                stroke_width: parse_f(at(&parts, 1)),
                layer_id: parse_i(at(&parts, 2)),
                raw_path: at(&parts, 4).unwrap_or("").to_string(),
            });
        }
        "RECT" => {
            if parts.len() < 9 {
                return;
            }
            fp.rectangles.push(EeFootprintRect {
                x: parse_f(at(&parts, 1)),
                y: parse_f(at(&parts, 2)),
                width: parse_f(at(&parts, 3)),
                height: parse_f(at(&parts, 4)),
                layer_id: parse_i(at(&parts, 5)),
                stroke_width: parse_f(at(&parts, 8)),
            });
        }
        "TEXT" => {
            if parts.len() < 11 {
                return;
            }
            let display = at(&parts, 11).map(|v| v != "0" && !v.is_empty()).unwrap_or(true);
            let font_size_str = at(&parts, 9).unwrap_or("5");
            let font_size = font_size_str
                .trim_end_matches("pt")
                .parse::<f64>()
                .unwrap_or(5.0);
            fp.texts.push(EeFootprintText {
                kind: at(&parts, 1).unwrap_or("").to_string(),
                center_x: parse_f(at(&parts, 2)),
                center_y: parse_f(at(&parts, 3)),
                stroke_width: parse_f(at(&parts, 4)),
                rotation_deg: parse_f(at(&parts, 5)),
                layer_id: parse_i(at(&parts, 7)),
                font_size,
                text: at(&parts, 10).unwrap_or("").to_string(),
                is_displayed: display,
            });
        }
        "SOLIDREGION" => {
            if parts.len() < 5 {
                return;
            }
            fp.solid_regions.push(EeFootprintSolidRegion {
                layer_id: parse_i(at(&parts, 1)),
                raw_path: at(&parts, 3).unwrap_or("").to_string(),
                region_type: at(&parts, 4).unwrap_or("solid").to_string(),
            });
        }
        "SVGNODE" => {
            if parts.len() >= 2 {
                if let Ok(v) = serde_json::from_str::<Value>(parts[1]) {
                    fp.model_3d = parse_svgnode_model(&v, canvas_ox, canvas_oy);
                }
            }
        }
        _ => {}
    }
}

fn parse_svgnode_model(node: &Value, canvas_ox: f64, canvas_oy: f64) -> Option<Ee3dModel> {
    let attrs = node.get("attrs")?;
    let uuid = safe_string(attrs.get("uuid").unwrap_or(&Value::Null));
    if uuid.is_empty() {
        return None;
    }
    let title = safe_string(attrs.get("title").unwrap_or(&Value::Null));
    let scale = 0.254_f64;
    let c_origin: Vec<f64> = safe_string(attrs.get("c_origin").unwrap_or(&Value::Null))
        .split(',')
        .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
        .collect();
    let c_ox = c_origin.first().copied().unwrap_or(0.0);
    let c_oy = c_origin.get(1).copied().unwrap_or(0.0);
    let tx = (c_ox - canvas_ox) * scale;
    let ty = -(c_oy - canvas_oy) * scale;
    let tz = safe_float(attrs.get("z").unwrap_or(&Value::Null)) * scale;

    let c_rot: Vec<f64> = safe_string(attrs.get("c_rotation").unwrap_or(&Value::Null))
        .split(',')
        .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
        .collect();

    Some(Ee3dModel {
        uuid,
        name: title,
        translation_x_mm: tx,
        translation_y_mm: ty,
        translation_z_mm: tz,
        rotation_x_deg: c_rot.first().copied().unwrap_or(0.0),
        rotation_y_deg: c_rot.get(1).copied().unwrap_or(0.0),
        rotation_z_deg: c_rot.get(2).copied().unwrap_or(0.0),
        raw_step: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_length_horizontal() {
        assert_eq!(derive_pin_length("M 400 300 h 20"), 20.0);
        assert_eq!(derive_pin_length("M 400 300 h -20"), 20.0);
        assert_eq!(derive_pin_length("M 0 0 v 10"), 10.0);
    }

    #[test]
    fn split_point_pairs() {
        let p = parse_point_pairs("0 0 5 5 -3 -3");
        assert_eq!(p, vec![(0.0, 0.0), (5.0, 5.0), (-3.0, -3.0)]);
    }

    #[test]
    fn sanitize_strips_packaging_suffix() {
        assert_eq!(sanitize_component_name("ATmega328P (TQFP)"), "ATmega328P");
        assert_eq!(sanitize_component_name("BC547[CutTape]"), "BC547");
        assert_eq!(sanitize_component_name("LM358"), "LM358");
    }
}
