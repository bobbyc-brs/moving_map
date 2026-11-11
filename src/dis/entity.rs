/// Entity status (APP-6A appurtenance code).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Friendly,
    Hostile,
    Unknown,
    Neutral,
}

impl Status {
    /// DIS Force ID (SISO-REF-010 UID 6): 1 Friendly, 2 Opposing, 3 Neutral;
    /// 0 (Other) and anything else display as Unknown.
    pub fn from_force_id(force_id: u8) -> Self {
        match force_id {
            1 => Self::Friendly,
            2 => Self::Hostile,
            3 => Self::Neutral,
            _ => Self::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Friendly => "F",
            Self::Hostile => "A",
            Self::Unknown => "U",
            Self::Neutral => "N",
        }
    }

    pub fn ansi_colour(&self) -> &'static str {
        match self {
            Self::Friendly => "\x1b[34m",
            Self::Hostile => "\x1b[31m",
            Self::Unknown => "\x1b[33m",
            Self::Neutral => "\x1b[32m",
        }
    }
}

/// DIS entity station.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Station {
    Infantry,
    Surface,
    Air,
    SurfaceVessel,
    Submarine,
    Installation,
    Equipment,
    Activity,
    CommandControl,
}

impl Station {
    pub fn from_station_id(value: u8) -> Self {
        match value {
            1 => Self::Infantry,
            2 => Self::Surface,
            3 => Self::Air,
            4 => Self::SurfaceVessel,
            5 => Self::Submarine,
            6 => Self::Installation,
            7 => Self::Equipment,
            8 => Self::Activity,
            _ => Self::CommandControl,
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            Self::Infantry => "I",
            Self::Surface => "S",
            Self::Air => "A",
            Self::SurfaceVessel => "V",
            Self::Submarine => "U",
            Self::Installation => "X",
            Self::Equipment => "E",
            Self::Activity => "D",
            Self::CommandControl => "C",
        }
    }
}

/// DIS entity activity.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Activity {
    Squad, Platoon, Company, Battalion, Brigade,
    Sedan, HpTruck, APC, Tank, Artillery,
    Helicopter, Fighter, Cargo, Tanker, Drone,
    SmallBoat, PatrolBoat, Frigate, Destroyer, Cruiser, Carrier,
    Ownship, Unknown,
}

impl Activity {
    pub fn from_activity_id(domain: u8, activity: u8) -> Self {
        match (domain, activity) {
            (_, 0) => Self::Unknown,
            (_, 255) => Self::Ownship,
            (0, 1) => Self::Squad,
            (0, 2) => Self::Platoon,
            (0, 3) => Self::Company,
            (0, 4) => Self::Battalion,
            (0, 5) => Self::Brigade,
            (0, 20) => Self::Sedan,
            (0, 21) => Self::HpTruck,
            (0, 22) => Self::APC,
            (0, 23) => Self::Tank,
            (0, 24) => Self::Artillery,
            (3, 1) => Self::Fighter,
            (3, 2) => Self::Helicopter,
            (3, 3) => Self::Cargo,
            (3, 4) => Self::Tanker,
            (3, 5) => Self::Drone,
            (2, 1) => Self::SmallBoat,
            (2, 2) => Self::PatrolBoat,
            (2, 3) => Self::Frigate,
            (2, 4) => Self::Destroyer,
            (2, 5) => Self::Cruiser,
            (2, 6) => Self::Carrier,
            _ => Self::Unknown,
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            Self::Squad => ".sq",
            Self::Platoon => "|sq",
            Self::Company => "||co",
            Self::Battalion => "=bat",
            Self::Brigade => "==brig",
            Self::Sedan => "sed",
            Self::HpTruck => "hpt",
            Self::APC => "apc",
            Self::Tank => "t",
            Self::Artillery => "fa",
            Self::Helicopter => "h",
            Self::Fighter => "f",
            Self::Cargo => "c",
            Self::Tanker => "tkr",
            Self::Drone => "u",
            Self::SmallBoat => "sb",
            Self::PatrolBoat => "pb",
            Self::Frigate => "ff",
            Self::Destroyer => "dd",
            Self::Cruiser => "cg",
            Self::Carrier => "cv",
            Self::Ownship => "OWN",
            Self::Unknown => "?",
        }
    }
}

/// DIS entity identifier.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub site: u16,
    pub application: u16,
    pub entity: u16,
}

impl EntityId {
    pub fn new(site: u16, application: u16, entity: u16) -> Self {
        Self { site, application, entity }
    }

    pub fn display(&self) -> String {
        format!("{}.{}.{}", self.site, self.application, self.entity)
    }
}

/// A tracked entity record.

#[derive(Debug, Clone)]
pub struct EntityRecord {
    pub entity_id: EntityId,
    pub name: String,
    pub station: Station,
    pub activity: Activity,
    pub status: Status,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub heading: f32,
    pub speed: f32,
    pub pitch: f32,
    pub roll: f32,
    pub last_heard: std::time::Instant,
}

impl EntityRecord {
    pub fn new(
        entity_id: EntityId,
        name: String,
        station: Station,
        activity: Activity,
        status: Status,
        latitude: f64,
        longitude: f64,
        altitude: f64,
        heading: f32,
        speed: f32,
        last_heard: std::time::Instant,
    ) -> Self {
        Self {
            entity_id, name, station, activity, status,
            latitude, longitude, altitude,
            heading, speed, pitch: 0.0, roll: 0.0,
            last_heard,
        }
    }

    pub fn is_stale(&self, timeout_secs: f64) -> bool {
        self.last_heard.elapsed().as_secs_f64() > timeout_secs
    }

    pub fn age_seconds(&self) -> f64 {
        self.last_heard.elapsed().as_secs_f64()
    }
}
