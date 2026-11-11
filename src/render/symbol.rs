//! DIS entity type → APP-6A symbol: battle dimension (frame shape) and icon.
//!
//! Frames follow APP-6A / MIL-STD-2525B. Military icons are simplified
//! (armour ellipse, rotary-wing bow-tie, …). Civilian vehicles and animals
//! have no APP-6A icon; they use local pictograms or text (DD-011).
//! Codes from SISO-REF-010 as checked in DD-004 / TESTLOG; country is ignored.

use crate::dis::packet::{domain, kind, EntityType};

/// Battle dimension — selects the frame shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Air,
    Ground,
    SeaSurface,
    Subsurface,
}

/// Icon drawn inside the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Armour,
    ArmouredFightingVehicle,
    ArmouredUtility,
    Artillery,
    MilitaryVehicle,
    Car,
    Bus,
    SchoolBus,
    Truck,
    CivilianVehicle,
    Infantry,
    Horse,
    HorseWithRider,
    Animal,
    AttackHelicopter,
    UtilityHelicopter,
    Helicopter,
    FixedWing,
    CivilAircraft,
    Uav,
    PatrolCraft,
    Ship,
    Submarine,
    Unknown,
}

impl Icon {
    /// Short text for the text map and for icons drawn as letters.
    pub fn code(self) -> &'static str {
        match self {
            Icon::Armour => "TK",
            Icon::ArmouredFightingVehicle => "AFV",
            Icon::ArmouredUtility => "AUV",
            Icon::Artillery => "ART",
            Icon::MilitaryVehicle => "MV",
            Icon::Car => "CAR",
            Icon::Bus => "BUS",
            Icon::SchoolBus => "SCH",
            Icon::Truck => "TRK",
            Icon::CivilianVehicle => "CIV",
            Icon::Infantry => "INF",
            Icon::Horse => "HRS",
            Icon::HorseWithRider => "HRS+R",
            Icon::Animal => "ANM",
            Icon::AttackHelicopter => "AH",
            Icon::UtilityHelicopter => "UH",
            Icon::Helicopter => "H",
            Icon::FixedWing => "FW",
            Icon::CivilAircraft => "CIV",
            Icon::Uav => "UAV",
            Icon::PatrolCraft => "PC",
            Icon::Ship => "SHP",
            Icon::Submarine => "SS",
            Icon::Unknown => "?",
        }
    }

    /// Plain-English description for the info panel.
    pub fn description(self) -> &'static str {
        match self {
            Icon::Armour => "Tank",
            Icon::ArmouredFightingVehicle => "Armoured fighting vehicle",
            Icon::ArmouredUtility => "Armoured utility vehicle",
            Icon::Artillery => "Self-propelled artillery",
            Icon::MilitaryVehicle => "Military land vehicle",
            Icon::Car => "Car",
            Icon::Bus => "Bus",
            Icon::SchoolBus => "School bus",
            Icon::Truck => "Cargo truck",
            Icon::CivilianVehicle => "Civilian vehicle",
            Icon::Infantry => "Dismounted person",
            Icon::Horse => "Horse",
            Icon::HorseWithRider => "Horse with rider",
            Icon::Animal => "Animal",
            Icon::AttackHelicopter => "Attack helicopter",
            Icon::UtilityHelicopter => "Utility helicopter",
            Icon::Helicopter => "Helicopter",
            Icon::FixedWing => "Fixed-wing aircraft",
            Icon::CivilAircraft => "Civil aircraft",
            Icon::Uav => "Uncrewed aircraft",
            Icon::PatrolCraft => "Patrol craft",
            Icon::Ship => "Surface vessel",
            Icon::Submarine => "Submarine",
            Icon::Unknown => "Unknown type",
        }
    }

    pub fn is_rotary(self) -> bool {
        matches!(self, Icon::AttackHelicopter | Icon::UtilityHelicopter | Icon::Helicopter)
    }
}

/// Symbol for a DIS entity type.
pub fn classify(t: &EntityType) -> (Dimension, Icon) {
    match (t.kind, t.domain) {
        (kind::PLATFORM, domain::LAND) => (Dimension::Ground, land_platform(t)),
        (kind::PLATFORM, domain::AIR) => (Dimension::Air, air_platform(t)),
        (kind::PLATFORM, domain::SURFACE) => (Dimension::SeaSurface, surface_platform(t)),
        (kind::PLATFORM, domain::SUBSURFACE) => (Dimension::Subsurface, Icon::Submarine),
        (kind::LIFE_FORM, domain::LAND) => (Dimension::Ground, land_life_form(t)),
        (kind::LIFE_FORM, domain::AIR) => (Dimension::Air, Icon::Unknown),
        (kind::LIFE_FORM, domain::SURFACE) => (Dimension::SeaSurface, Icon::Infantry),
        (kind::LIFE_FORM, domain::SUBSURFACE) => (Dimension::Subsurface, Icon::Infantry),
        (_, domain::AIR) => (Dimension::Air, Icon::Unknown),
        (_, domain::SURFACE) => (Dimension::SeaSurface, Icon::Unknown),
        (_, domain::SUBSURFACE) => (Dimension::Subsurface, Icon::Unknown),
        _ => (Dimension::Ground, Icon::Unknown),
    }
}

fn land_platform(t: &EntityType) -> Icon {
    match (t.category, t.subcategory) {
        (1, _) => Icon::Armour,
        (2, _) => Icon::ArmouredFightingVehicle,
        (3, _) => Icon::ArmouredUtility,
        (4, _) => Icon::Artillery,
        (81, _) => Icon::Car,
        (82, 12) => Icon::SchoolBus,
        (82, _) => Icon::Bus,
        (83..=85, _) => Icon::Truck,
        (80..=99, _) => Icon::CivilianVehicle,
        (5..=79, _) => Icon::MilitaryVehicle,
        _ => Icon::Unknown,
    }
}

fn air_platform(t: &EntityType) -> Icon {
    match t.category {
        20 => Icon::AttackHelicopter,
        21 => Icon::UtilityHelicopter,
        22..=29 | 90..=99 => Icon::Helicopter,
        50..=59 => Icon::Uav,
        80..=89 => Icon::CivilAircraft,
        1..=19 | 30..=49 | 60..=79 => Icon::FixedWing,
        _ => Icon::Unknown,
    }
}

fn surface_platform(t: &EntityType) -> Icon {
    match t.category {
        7 => Icon::PatrolCraft,
        0 => Icon::Unknown,
        _ => Icon::Ship,
    }
}

fn land_life_form(t: &EntityType) -> Icon {
    match (t.category, t.subcategory) {
        (200, 80) => Icon::Horse,
        (200, 90) => Icon::HorseWithRider,
        (200..=255, _) => Icon::Animal,
        _ => Icon::Infantry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(kind: u8, domain: u8, category: u8, subcategory: u8) -> EntityType {
        EntityType { kind, domain, country: 39, category, subcategory, ..EntityType::default() }
    }

    /// Every type in Sensors' ottawa-mixed scenario (tests/data/dis) has a symbol.
    #[test]
    fn ottawa_mixed_types() {
        let cases = [
            (t(1, 1, 1, 0), Dimension::Ground, Icon::Armour),
            (t(1, 1, 2, 0), Dimension::Ground, Icon::ArmouredFightingVehicle),
            (t(1, 1, 81, 10), Dimension::Ground, Icon::Car),
            (t(1, 1, 82, 0), Dimension::Ground, Icon::Bus),
            (t(1, 1, 82, 12), Dimension::Ground, Icon::SchoolBus),
            (t(1, 1, 83, 0), Dimension::Ground, Icon::Truck),
            (t(1, 2, 20, 0), Dimension::Air, Icon::AttackHelicopter),
            (t(1, 2, 21, 0), Dimension::Air, Icon::UtilityHelicopter),
            (t(1, 2, 87, 0), Dimension::Air, Icon::CivilAircraft),
            (t(1, 3, 7, 0), Dimension::SeaSurface, Icon::PatrolCraft),
            (t(3, 1, 200, 80), Dimension::Ground, Icon::Horse),
            (t(3, 1, 200, 90), Dimension::Ground, Icon::HorseWithRider),
        ];
        for (ty, dim, icon) in cases {
            assert_eq!(classify(&ty), (dim, icon), "{}", ty.display());
        }
    }

    #[test]
    fn unknown_types_still_get_a_frame() {
        assert_eq!(classify(&t(0, 0, 0, 0)), (Dimension::Ground, Icon::Unknown));
        assert_eq!(classify(&t(9, 2, 0, 0)), (Dimension::Air, Icon::Unknown));
        assert_eq!(classify(&t(1, 4, 0, 0)), (Dimension::Subsurface, Icon::Submarine));
    }
}
