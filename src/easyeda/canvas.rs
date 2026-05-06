//! Coordinate transforms between EasyEDA pixel space and Altium [`Coord`].
//!
//! 1 EasyEDA pixel = 0.254 mm = 10 mil.

use altium::coord::Coord;

/// EasyEDA pixel coords are integer multiples of 5 px (= 50 mil = 1.27 mm) for
/// pin endpoints. Snap a bbox origin to the 5-px grid so pin coordinates land
/// on the 100-mil grid after subtraction.
pub fn snap_to_5px_grid(value: f64) -> f64 {
    (value / 5.0).round() * 5.0
}

/// 1 EE pixel = 10 mil → `Coord::from_mils(px * 10)`.
pub fn ee_px_to_coord(px: f64) -> Coord {
    Coord::from_mils(px * 10.0)
}

/// EE → mm (used for 3D model translation).
pub fn ee_px_to_mm(px: f64) -> f64 {
    px * 0.254
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_5px() {
        assert_eq!(snap_to_5px_grid(7.3), 5.0);
        assert_eq!(snap_to_5px_grid(8.0), 10.0);
        assert_eq!(snap_to_5px_grid(-3.0), -5.0);
    }

    #[test]
    fn one_px_is_ten_mils() {
        assert_eq!(ee_px_to_coord(1.0), Coord::from_mils(10.0));
    }
}
