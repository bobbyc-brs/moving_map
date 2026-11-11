/// NATO APP-6A symbol lookup — character representation.
///
/// The egui painter functions were removed to achieve zero external
/// dependencies. The character lookup remains for the text renderer.

use crate::dis::entity::{Activity, EntityRecord, Station};

/// Render a single entity character for the text map.
pub fn entity_char(record: &EntityRecord) -> char {
    match record.station {
        Station::Infantry => match record.activity {
            Activity::Squad => '.',
            Activity::Platoon => '|',
            Activity::Company => '|',
            Activity::Battalion => '=',
            Activity::Brigade => '=',
            _ => '.',
        },
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
            Activity::SmallBoat => 's',
            Activity::PatrolBoat => 'p',
            Activity::Frigate => 'F',
            Activity::Destroyer => 'D',
            Activity::Cruiser => 'C',
            Activity::Carrier => 'X',
            _ => 'V',
        },
        Station::Submarine => 'U',
        Station::Installation => 'X',
        Station::Equipment => 'e',
        Station::Activity => 'd',
        Station::CommandControl => 'C',
    }
}

/// Character for ownship.
pub const OWNSHIP_CHAR: char = 'o';

/// Render a full NATO symbol description string for the info panel.
pub fn symbol_description(record: &EntityRecord) -> String {
    format!(
        "{} {} {} {} ({}.{}.{})",
        record.status.label(),
        record.station.short(),
        record.activity.short(),
        entity_char(record),
        record.entity_id.site,
        record.entity_id.application,
        record.entity_id.entity,
    )
}
