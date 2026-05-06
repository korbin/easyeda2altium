use altium::coord::{Coord, CoordPoint};
use altium::enums::{PinElectricalType, PinOrientation, TextJustification};
use altium::sch::primitives::PrimitiveCommon;
use altium::sch::{
    Arc, Component, Ellipse, Implementation, Parameter, Pin, Polyline, Rectangle,
};

use crate::easyeda::canvas::snap_to_5px_grid;
use crate::easyeda::types::{
    EePinElectrical, EeSymbol, EeSymbolArc, EeSymbolCircle, EeSymbolEllipse, EeSymbolPin,
    EeSymbolPolyline, EeSymbolRectangle,
};

const ALTIUM_BODY_FILL_BGR: i32 = 0x00B0_FFFF;
const ALTIUM_BODY_BORDER_BGR: i32 = 0;

pub fn convert_symbol(
    ee: &EeSymbol,
    custom_fields: &[(String, String)],
    use_custom_pin_font: bool,
) -> Component {
    let font_id_for_text: i32 = if use_custom_pin_font { 2 } else { 1 };
    let origin_x = snap_to_5px_grid(ee.bbox.x);
    let origin_y = snap_to_5px_grid(ee.bbox.y);

    let mut comp = Component::new(ee.info.name.clone());

    let description = if !ee.info.description.is_empty() {
        ee.info.description.clone()
    } else if !ee.info.mpn.is_empty() && !ee.info.manufacturer.is_empty() {
        format!("{} {}", ee.info.manufacturer, ee.info.mpn)
    } else if !ee.info.mpn.is_empty() {
        ee.info.mpn.clone()
    } else {
        String::new()
    };
    comp.description = if description.is_empty() {
        None
    } else {
        Some(description.clone())
    };

    let raw_prefix = ee.info.prefix.replace('?', "");
    comp.designator_prefix = if raw_prefix.is_empty() {
        None
    } else {
        Some(raw_prefix.clone())
    };
    comp.lib_reference = Some(ee.info.name.clone());
    comp.design_item_id = Some(format!("{}-{}", ee.info.name, ee.info.lcsc_id));

    comp.area_color = ALTIUM_BODY_FILL_BGR;
    comp.color = ALTIUM_BODY_BORDER_BGR;
    comp.part_id_locked = true;

    let units: Vec<&EeSymbol> = std::iter::once(ee).chain(ee.sub_symbols.iter()).collect();
    comp.part_count = units.len() as i32;
    comp.current_part_id = 1;
    comp.display_mode_count = 1;

    for (i, unit) in units.iter().enumerate() {
        let owner_part_id = (i + 1) as i32;
        emit_unit(
            unit,
            owner_part_id,
            origin_x,
            origin_y,
            use_custom_pin_font,
            font_id_for_text,
            &mut comp,
        );
    }

    comp.add_default_labels(&raw_prefix, font_id_for_text);

    for (k, v) in builtin_parameters(ee).iter().chain(custom_fields.iter()) {
        comp.parameters.push(make_parameter(k, v, font_id_for_text));
    }

    if !ee.info.package.is_empty() {
        comp.implementations.push(make_implementation(&ee.info.package));
    }

    comp
}

fn emit_unit(
    unit: &EeSymbol,
    owner_part_id: i32,
    origin_x: f64,
    origin_y: f64,
    use_custom_pin_font: bool,
    font_id_for_text: i32,
    comp: &mut Component,
) {
    let unit_pin_index_start = comp.pins.len();
    for pin in &unit.pins {
        comp.pins.push(convert_pin(
            pin,
            owner_part_id,
            origin_x,
            origin_y,
            use_custom_pin_font,
            font_id_for_text,
        ));
    }
    let pin_bbox = pins_bbox(&comp.pins[unit_pin_index_start..]);
    for r in &unit.rectangles {
        comp.rectangles.push(convert_rectangle(
            r,
            owner_part_id,
            origin_x,
            origin_y,
            pin_bbox,
        ));
    }
    for c in &unit.circles {
        comp.ellipses
            .push(convert_circle(c, owner_part_id, origin_x, origin_y));
    }
    for e in &unit.ellipses {
        comp.ellipses
            .push(convert_ellipse(e, owner_part_id, origin_x, origin_y));
    }
    for p in &unit.polylines {
        comp.polylines
            .push(convert_polyline(p, owner_part_id, origin_x, origin_y));
    }
    for arc in &unit.arcs {
        if let Some(a) = convert_arc(arc, owner_part_id, origin_x, origin_y) {
            comp.arcs.push(a);
        }
    }
}

fn convert_pin(
    pin: &EeSymbolPin,
    owner_part_id: i32,
    ox: f64,
    oy: f64,
    use_custom_pin_font: bool,
    font_id_for_text: i32,
) -> Pin {
    let (font_mode, custom_font_id) = if use_custom_pin_font {
        (2, font_id_for_text)
    } else {
        (0, 0)
    };
    Pin {
        name: if pin.name.is_empty() {
            None
        } else {
            Some(pin.name.clone())
        },
        designator: if pin.designator.is_empty() {
            None
        } else {
            Some(pin.designator.clone())
        },
        location: CoordPoint::new(
            ee_px_to_coord(pin.pos_x - ox),
            -ee_px_to_coord(pin.pos_y - oy),
        ),
        length: Coord::from_mils(200.0),
        electrical_type: ee_pin_electrical(pin.electrical),
        orientation: ee_orientation(pin.rotation_deg),
        show_name: !pin.name.is_empty(),
        show_designator: !pin.designator.is_empty(),
        is_hidden: !pin.is_displayed,
        symbol_inner_edge: if pin.is_clock { 3 } else { 0 },
        symbol_outer_edge: if pin.is_inverted { 1 } else { 0 },
        designator_font_mode: font_mode,
        designator_custom_font_id: custom_font_id,
        name_font_mode: font_mode,
        name_custom_font_id: custom_font_id,
        common: with_owner(owner_part_id),
        ..Default::default()
    }
}

fn pins_bbox(pins: &[Pin]) -> Option<(Coord, Coord, Coord, Coord)> {
    if pins.is_empty() {
        return None;
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for p in pins {
        let (x, y) = (p.location.x.to_raw(), p.location.y.to_raw());
        if x < min_x { min_x = x; }
        if x > max_x { max_x = x; }
        if y < min_y { min_y = y; }
        if y > max_y { max_y = y; }
    }
    Some((
        Coord::from_raw(min_x),
        Coord::from_raw(min_y),
        Coord::from_raw(max_x),
        Coord::from_raw(max_y),
    ))
}

fn convert_rectangle(
    r: &EeSymbolRectangle,
    owner_part_id: i32,
    ox: f64,
    oy: f64,
    pin_bbox: Option<(Coord, Coord, Coord, Coord)>,
) -> Rectangle {
    let lx_ee = ee_px_to_coord(r.pos_x - ox);
    let ty_ee = -ee_px_to_coord(r.pos_y - oy);
    let rx_ee = ee_px_to_coord(r.pos_x - ox + r.width);
    let by_ee = -ee_px_to_coord(r.pos_y - oy + r.height);
    let mut lx = lx_ee.min(rx_ee);
    let mut rx = lx_ee.max(rx_ee);
    let mut by = by_ee.min(ty_ee);
    let mut ty = by_ee.max(ty_ee);
    if let Some((pmin_x, pmin_y, pmax_x, pmax_y)) = pin_bbox {
        if pmin_x < lx { lx = pmin_x; }
        if pmax_x > rx { rx = pmax_x; }
        if pmin_y < by { by = pmin_y; }
        if pmax_y > ty { ty = pmax_y; }
    }
    Rectangle {
        corner1: CoordPoint::new(lx, by),
        corner2: CoordPoint::new(rx, ty),
        is_filled: true,
        is_transparent: false,
        color: ALTIUM_BODY_BORDER_BGR,
        fill_color: ALTIUM_BODY_FILL_BGR,
        line_width: Coord::from_mils(2.0),
        line_style: 0,
        common: with_owner(owner_part_id),
    }
}

fn convert_circle(c: &EeSymbolCircle, owner_part_id: i32, ox: f64, oy: f64) -> Ellipse {
    Ellipse {
        center: CoordPoint::new(
            ee_px_to_coord(c.center_x - ox),
            -ee_px_to_coord(c.center_y - oy),
        ),
        radius_x: ee_px_to_coord(c.radius),
        radius_y: ee_px_to_coord(c.radius),
        is_filled: c.fill,
        color: ALTIUM_BODY_BORDER_BGR,
        fill_color: ALTIUM_BODY_FILL_BGR,
        common: with_owner(owner_part_id),
        ..Default::default()
    }
}

fn convert_ellipse(e: &EeSymbolEllipse, owner_part_id: i32, ox: f64, oy: f64) -> Ellipse {
    Ellipse {
        center: CoordPoint::new(
            ee_px_to_coord(e.center_x - ox),
            -ee_px_to_coord(e.center_y - oy),
        ),
        radius_x: ee_px_to_coord(e.radius_x),
        radius_y: ee_px_to_coord(e.radius_y),
        is_filled: e.fill,
        color: ALTIUM_BODY_BORDER_BGR,
        fill_color: ALTIUM_BODY_FILL_BGR,
        common: with_owner(owner_part_id),
        ..Default::default()
    }
}

fn convert_polyline(p: &EeSymbolPolyline, owner_part_id: i32, ox: f64, oy: f64) -> Polyline {
    let vertices = p
        .points
        .iter()
        .map(|&(x, y)| CoordPoint::new(ee_px_to_coord(x - ox), -ee_px_to_coord(y - oy)))
        .collect();
    Polyline {
        vertices,
        color: ALTIUM_BODY_BORDER_BGR,
        common: with_owner(owner_part_id),
        ..Default::default()
    }
}

fn convert_arc(arc: &EeSymbolArc, owner_part_id: i32, ox: f64, oy: f64) -> Option<Arc> {
    let svg = parse_svg_arc(&arc.raw_path)?;
    let (cx, cy, r) = svg_arc_center(&svg)?;
    Some(Arc {
        center: CoordPoint::new(ee_px_to_coord(cx - ox), -ee_px_to_coord(cy - oy)),
        radius: ee_px_to_coord(r),
        start_angle: 0.0,
        end_angle: 360.0,
        color: ALTIUM_BODY_BORDER_BGR,
        common: with_owner(owner_part_id),
        ..Default::default()
    })
}

fn make_parameter(name: &str, value: &str, font_id: i32) -> Parameter {
    Parameter {
        name: name.to_string(),
        value: value.to_string(),
        is_visible: false,
        font_id,
        common: PrimitiveCommon {
            owner_part_id: -1,
            is_not_accessible: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn make_implementation(footprint_name: &str) -> Implementation {
    Implementation {
        model_name: Some(footprint_name.to_string()),
        model_type: Some("PCBLIB".to_string()),
        is_current: true,
        ..Default::default()
    }
}

fn ee_pin_electrical(e: EePinElectrical) -> PinElectricalType {
    match e {
        EePinElectrical::Input => PinElectricalType::Input,
        EePinElectrical::Output => PinElectricalType::Output,
        EePinElectrical::Bidirectional => PinElectricalType::InputOutput,
        EePinElectrical::Power => PinElectricalType::Power,
        EePinElectrical::Unspecified => PinElectricalType::Passive,
    }
}

fn ee_orientation(rotation_deg: i32) -> PinOrientation {
    match ((rotation_deg % 360 + 360) % 360) / 90 {
        0 => PinOrientation::Right,
        1 => PinOrientation::Up,
        2 => PinOrientation::Left,
        _ => PinOrientation::Down,
    }
}

fn with_owner(owner_part_id: i32) -> PrimitiveCommon {
    PrimitiveCommon {
        owner_part_id,
        is_not_accessible: true,
        ..Default::default()
    }
}

fn ee_px_to_coord(px: f64) -> Coord {
    Coord::from_mils(px * 10.0)
}

fn builtin_parameters(ee: &EeSymbol) -> Vec<(String, String)> {
    let mut v = Vec::new();
    if !ee.info.manufacturer.is_empty() {
        v.push(("Manufacturer".to_string(), ee.info.manufacturer.clone()));
    }
    if !ee.info.mpn.is_empty() {
        v.push(("MPN".to_string(), ee.info.mpn.clone()));
    }
    if !ee.info.lcsc_id.is_empty() {
        v.push(("LCSC Part".to_string(), ee.info.lcsc_id.clone()));
    }
    if !ee.info.datasheet.is_empty() {
        v.push(("Datasheet".to_string(), ee.info.datasheet.clone()));
    }
    if !ee.info.keywords.is_empty() {
        v.push(("Keywords".to_string(), ee.info.keywords.clone()));
    }
    let _ = TextJustification::BottomLeft;
    v
}

#[derive(Debug, Clone, Copy)]
struct SvgArc {
    sx: f64,
    sy: f64,
    rx: f64,
    ry: f64,
    large: bool,
    sweep: bool,
    ex: f64,
    ey: f64,
}

fn parse_svg_arc(path: &str) -> Option<SvgArc> {
    let toks: Vec<&str> = path
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect();
    let mut iter = toks.into_iter();
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;
    let mut have_m = false;
    while let Some(t) = iter.next() {
        match t {
            "M" => {
                sx = iter.next()?.parse().ok()?;
                sy = iter.next()?.parse().ok()?;
                have_m = true;
            }
            "A" if have_m => {
                let rx: f64 = iter.next()?.parse().ok()?;
                let ry: f64 = iter.next()?.parse().ok()?;
                let _phi: f64 = iter.next()?.parse().ok()?;
                let large_s: i32 = iter.next()?.parse().ok()?;
                let sweep_s: i32 = iter.next()?.parse().ok()?;
                let ex: f64 = iter.next()?.parse().ok()?;
                let ey: f64 = iter.next()?.parse().ok()?;
                return Some(SvgArc {
                    sx,
                    sy,
                    rx,
                    ry,
                    large: large_s != 0,
                    sweep: sweep_s != 0,
                    ex,
                    ey,
                });
            }
            _ => {}
        }
    }
    None
}

fn svg_arc_center(a: &SvgArc) -> Option<(f64, f64, f64)> {
    let rx = a.rx.abs();
    let ry = a.ry.abs();
    if rx == 0.0 || ry == 0.0 {
        return None;
    }
    let dx2 = (a.sx - a.ex) / 2.0;
    let dy2 = (a.sy - a.ey) / 2.0;
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x1_sq = dx2 * dx2;
    let y1_sq = dy2 * dy2;
    let mut radii_check = x1_sq / rx_sq + y1_sq / ry_sq;
    let (rx_eff, ry_eff) = if radii_check > 1.0 {
        radii_check = radii_check.sqrt();
        (rx * radii_check, ry * radii_check)
    } else {
        (rx, ry)
    };
    let rx_sq = rx_eff * rx_eff;
    let ry_sq = ry_eff * ry_eff;
    let den = rx_sq * y1_sq + ry_sq * x1_sq;
    if den == 0.0 {
        return None;
    }
    let num = (rx_sq * ry_sq - rx_sq * y1_sq - ry_sq * x1_sq).max(0.0);
    let sign = if a.large == a.sweep { -1.0 } else { 1.0 };
    let coef = sign * (num / den).sqrt();
    let cx1 = coef * (rx_eff * dy2 / ry_eff);
    let cy1 = coef * -(ry_eff * dx2 / rx_eff);
    let cx = cx1 + (a.sx + a.ex) / 2.0;
    let cy = cy1 + (a.sy + a.ey) / 2.0;
    let r = ((cx - a.sx).powi(2) + (cy - a.sy).powi(2)).sqrt();
    Some((cx, cy, r))
}
