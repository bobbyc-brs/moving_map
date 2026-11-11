/// Text-map symbols. Entities received over DIS are drawn from their entity
/// type (`render::symbol`); the seeded demo entities, which have no DIS type,
/// fall back to their station / activity.

use crate::dis::entity::{Activity, EntityRecord, Station};
use crate::render::symbol::{classify, Icon};

/// Render a single entity character for the text map.
pub fn entity_char(record: &EntityRecord) -> char {
    if record.entity_type.kind != 0 {
        return icon_char(classify(&record.entity_type).1);
    }
    match record.station {
        Station::Infantry => '.',
        Station::Surface => match record.activity {
            Activity::Sedan | Activity::HpTruck => 'T',
            Activity::APC => 'A',
            Activity::Tank => 'T',
            Activity::Artillery => 'F',
            _ => 'S',
        },
        Station::Air => match record.activity {
            Activity::Fighter => 'f',
            Activity::Helicopter => 'h',
            Activity::Cargo => 'C',
            Activity::Tanker => 'K',
            Activity::Drone => 'u',
            _ => 'A',
        },
        Station::SurfaceVessel => match record.activity {
            Activity::PatrolBoat => 'p',
            _ => 'V',
        },
        Station::Submarine => 'U',
        Station::Installation => 'X',
        Station::Equipment => 'e',
        Station::Activity => 'd',
        Station::CommandControl => 'C',
    }
}

/// One character per icon for the 80-column text map.
pub fn icon_char(icon: Icon) -> char {
    match icon {
        Icon::Armour => 'T',
        Icon::ArmouredFightingVehicle | Icon::ArmouredUtility => 'A',
        Icon::Artillery => 'F',
        Icon::MilitaryVehicle => 'M',
        Icon::Car => 'c',
        Icon::Bus | Icon::SchoolBus => 'b',
        Icon::Truck => 't',
        Icon::CivilianVehicle => 'v',
        Icon::Infantry => 'i',
        Icon::Horse | Icon::HorseWithRider | Icon::Animal => 'r',
        Icon::AttackHelicopter | Icon::UtilityHelicopter | Icon::Helicopter => 'h',
        Icon::FixedWing => 'f',
        Icon::CivilAircraft => 'a',
        Icon::Uav => 'u',
        Icon::PatrolCraft => 'p',
        Icon::Ship => 'V',
        Icon::Submarine => 'U',
        Icon::Unknown => '?',
    }
}

/// Character for ownship.
pub const OWNSHIP_CHAR: char = 'o';
