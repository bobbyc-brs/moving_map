/// DIS PDU parsing — IEEE 1278.1 wire format, protocol versions 6 and 7.
///
/// Layouts verified 2026-11-10 against two independent implementations
/// (Wireshark `epan/dissectors/packet-dis.c`, Open-DIS `opendis/dis7.py`);
/// see DD-004. Every multi-byte field is **big-endian** (network order).

use crate::dis::entity::{Activity, EntityId, EntityRecord, Station, Status};
use crate::map::projection::ecef_to_geodetic;

/// PDU type values (header byte 2).
pub mod pdu_type {
    pub const ENTITY_STATE: u8 = 1;
    pub const FIRE: u8 = 2;
    pub const DETONATION: u8 = 3;
    pub const COLLISION: u8 = 4;
    pub const CREATE_ENTITY: u8 = 11;
    pub const REMOVE_ENTITY: u8 = 12;
    pub const ENTITY_STATE_UPDATE: u8 = 67;
}

/// Protocol family values (header byte 3).
pub mod family {
    pub const ENTITY_INFORMATION: u8 = 1;
    pub const WARFARE: u8 = 2;
    pub const SIMULATION_MANAGEMENT: u8 = 5;
}

/// Entity kind values (entity type record, byte 0).
pub mod kind {
    pub const PLATFORM: u8 = 1;
    pub const LIFE_FORM: u8 = 3;
    pub const CULTURAL_FEATURE: u8 = 5;
}

/// Platform domain values (entity type record, byte 1).
pub mod domain {
    pub const LAND: u8 = 1;
    pub const AIR: u8 = 2;
    pub const SURFACE: u8 = 3;
    pub const SUBSURFACE: u8 = 4;
    pub const SPACE: u8 = 5;
}

pub const HEADER_LEN: usize = 12;
/// Entity State PDU with zero variable parameter records.
pub const ENTITY_STATE_MIN_LEN: usize = 144;
pub const VARIABLE_PARAMETER_LEN: usize = 16;
pub const REMOVE_ENTITY_LEN: usize = 28;

// ---------------------------------------------------------------------------
// Errors and byte readers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Truncated { expected: usize, actual: usize },
    InvalidLength(u16),
    WrongPduType { expected: u8, actual: u8 },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { expected, actual } => {
                write!(f, "PDU too short: expected {expected}, got {actual}")
            }
            Self::InvalidLength(len) => write!(f, "invalid PDU length field: {len}"),
            Self::WrongPduType { expected, actual } => {
                write!(f, "expected PDU type {expected}, got {actual}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn need(buf: &[u8], expected: usize) -> Result<(), ParseError> {
    if buf.len() < expected {
        return Err(ParseError::Truncated { expected, actual: buf.len() });
    }
    Ok(())
}

fn be_u16(b: &[u8], at: usize) -> u16 { u16::from_be_bytes([b[at], b[at + 1]]) }
fn be_u32(b: &[u8], at: usize) -> u32 { u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) }
fn be_f32(b: &[u8], at: usize) -> f32 { f32::from_be_bytes(b[at..at + 4].try_into().unwrap()) }
fn be_f64(b: &[u8], at: usize) -> f64 { f64::from_be_bytes(b[at..at + 8].try_into().unwrap()) }

fn entity_id_at(b: &[u8], at: usize) -> EntityId {
    EntityId::new(be_u16(b, at), be_u16(b, at + 2), be_u16(b, at + 4))
}

// ---------------------------------------------------------------------------
// PDU header — 12 bytes
// ---------------------------------------------------------------------------
//
//  0 protocol version   1 exercise ID   2 PDU type   3 protocol family
//  4 timestamp (u32)    8 length (u16, whole PDU in bytes)
// 10 PDU status (v7; padding in v6)    11 padding

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PduHeader {
    pub protocol_version: u8,
    pub exercise_id: u8,
    pub pdu_type: u8,
    pub protocol_family: u8,
    pub timestamp: u32,
    pub length: u16,
    pub pdu_status: u8,
}

impl PduHeader {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        need(buf, HEADER_LEN)?;
        let length = be_u16(buf, 8);
        if (length as usize) < HEADER_LEN {
            return Err(ParseError::InvalidLength(length));
        }
        Ok(Self {
            protocol_version: buf[0],
            exercise_id: buf[1],
            pdu_type: buf[2],
            protocol_family: buf[3],
            timestamp: be_u32(buf, 4),
            length,
            pdu_status: buf[10],
        })
    }

    fn expect_type(buf: &[u8], expected: u8) -> Result<Self, ParseError> {
        let header = Self::parse(buf)?;
        if header.pdu_type != expected {
            return Err(ParseError::WrongPduType { expected, actual: header.pdu_type });
        }
        Ok(header)
    }
}

// ---------------------------------------------------------------------------
// Entity type record — 8 bytes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntityType {
    pub kind: u8,
    pub domain: u8,
    pub country: u16,
    pub category: u8,
    pub subcategory: u8,
    pub specific: u8,
    pub extra: u8,
}

impl EntityType {
    fn at(b: &[u8], at: usize) -> Self {
        Self {
            kind: b[at],
            domain: b[at + 1],
            country: be_u16(b, at + 2),
            category: b[at + 4],
            subcategory: b[at + 5],
            specific: b[at + 6],
            extra: b[at + 7],
        }
    }

    /// Map kind/domain onto this program's display station.
    pub fn station(&self) -> Station {
        match (self.kind, self.domain) {
            (kind::PLATFORM, domain::LAND) => Station::Surface,
            (kind::PLATFORM, domain::AIR) => Station::Air,
            (kind::PLATFORM, domain::SURFACE) => Station::SurfaceVessel,
            (kind::PLATFORM, domain::SUBSURFACE) => Station::Submarine,
            (kind::PLATFORM, domain::SPACE) => Station::Equipment,
            (kind::LIFE_FORM, _) => Station::Infantry,
            (kind::CULTURAL_FEATURE, _) => Station::Installation,
            _ => Station::CommandControl,
        }
    }

    /// Display as `kind.domain.country.category.subcategory.specific.extra`.
    pub fn display(&self) -> String {
        format!("{}.{}.{}.{}.{}.{}.{}", self.kind, self.domain, self.country,
            self.category, self.subcategory, self.specific, self.extra)
    }
}

// ---------------------------------------------------------------------------
// Entity State PDU — type 1, family 1, 144 bytes + 16 per variable parameter
// ---------------------------------------------------------------------------
//
//  12 entity ID (3×u16)        18 force ID          19 variable parameter count
//  20 entity type (8)          28 alternative entity type (8)
//  36 linear velocity (3×f32, m/s, world/ECEF frame)
//  48 location (3×f64, metres, geocentric ECEF)
//  72 orientation (3×f32 psi/theta/phi, radians, relative to ECEF)
//  84 appearance (u32)         88 dead-reckoning parameters (40)
// 128 marking (charset u8 + 11 chars)   140 capabilities (u32)
// 144 variable parameter records (16 each)

#[derive(Debug, Clone, PartialEq)]
pub struct EntityStatePdu {
    pub header: PduHeader,
    pub entity_id: EntityId,
    pub force_id: u8,
    pub variable_parameter_count: u8,
    pub entity_type: EntityType,
    pub linear_velocity: [f32; 3],
    pub location: [f64; 3],
    pub orientation: [f32; 3],
    pub appearance: u32,
    pub marking: String,
    pub capabilities: u32,
}

impl EntityStatePdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::expect_type(buf, pdu_type::ENTITY_STATE)?;
        need(buf, ENTITY_STATE_MIN_LEN)?;
        let variable_parameter_count = buf[19];
        need(buf, ENTITY_STATE_MIN_LEN + VARIABLE_PARAMETER_LEN * variable_parameter_count as usize)?;

        // Marking: byte 128 is the character set; 11 bytes of text follow,
        // NUL- or space-padded.
        let marking = buf[129..140]
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| if c.is_ascii_graphic() || c == b' ' { c as char } else { '?' })
            .collect::<String>()
            .trim_end()
            .to_string();

        Ok(Self {
            header,
            entity_id: entity_id_at(buf, 12),
            force_id: buf[18],
            variable_parameter_count,
            entity_type: EntityType::at(buf, 20),
            linear_velocity: [be_f32(buf, 36), be_f32(buf, 40), be_f32(buf, 44)],
            location: [be_f64(buf, 48), be_f64(buf, 56), be_f64(buf, 64)],
            orientation: [be_f32(buf, 72), be_f32(buf, 76), be_f32(buf, 80)],
            appearance: be_u32(buf, 84),
            marking,
            capabilities: be_u32(buf, 140),
        })
    }

    /// WGS84 latitude (°), longitude (°), and ellipsoid height (m).
    pub fn geodetic(&self) -> (f64, f64, f64) {
        let [x, y, z] = self.location;
        ecef_to_geodetic(x, y, z)
    }

    /// Ground-frame speed in m/s (magnitude of the ECEF velocity vector).
    pub fn speed(&self) -> f32 {
        let [vx, vy, vz] = self.linear_velocity;
        (vx * vx + vy * vy + vz * vz).sqrt()
    }

    /// True heading in degrees [0, 360): the body x-axis, given by psi and
    /// theta in the ECEF frame, projected onto the local north/east plane.
    pub fn heading_degrees(&self) -> f32 {
        let (lat, lon, _) = self.geodetic();
        heading_from_orientation(lat, lon, self.orientation[0] as f64, self.orientation[1] as f64) as f32
    }

    /// Build the display record. `now` stamps the last-heard time.
    pub fn to_record(&self, now: std::time::Instant) -> EntityRecord {
        let (lat, lon, alt) = self.geodetic();
        let name = if self.marking.is_empty() {
            self.entity_id.display()
        } else {
            self.marking.clone()
        };
        EntityRecord::new(
            self.entity_id,
            name,
            self.entity_type.station(),
            Activity::Unknown, // needs SISO-REF-010 category tables — see PLAN.md
            Status::from_force_id(self.force_id),
            lat,
            lon,
            alt,
            self.heading_degrees(),
            self.speed(),
            now,
        )
    }
}

/// Heading (°, true) of a body whose DIS Euler angles psi and theta are
/// relative to ECEF, at the given geodetic position.
pub fn heading_from_orientation(lat_deg: f64, lon_deg: f64, psi: f64, theta: f64) -> f64 {
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    // Body x-axis in ECEF for a z-y-x Euler sequence.
    let body_x = [theta.cos() * psi.cos(), theta.cos() * psi.sin(), -theta.sin()];
    let north = [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()];
    let east = [-lon.sin(), lon.cos(), 0.0];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let h = dot(body_x, east).atan2(dot(body_x, north)).to_degrees();
    if h < 0.0 { h + 360.0 } else { h }
}

// ---------------------------------------------------------------------------
// Remove Entity PDU — type 12, family 5, 28 bytes
// ---------------------------------------------------------------------------
//
// 12 originating entity ID   18 receiving entity ID   24 request ID (u32)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemoveEntityPdu {
    pub header: PduHeader,
    pub originating: EntityId,
    pub receiving: EntityId,
    pub request_id: u32,
}

impl RemoveEntityPdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::expect_type(buf, pdu_type::REMOVE_ENTITY)?;
        need(buf, REMOVE_ENTITY_LEN)?;
        Ok(Self {
            header,
            originating: entity_id_at(buf, 12),
            receiving: entity_id_at(buf, 18),
            request_id: be_u32(buf, 24),
        })
    }
}

// ---------------------------------------------------------------------------
// Dispatch and datagram splitting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Pdu {
    EntityState(EntityStatePdu),
    RemoveEntity(RemoveEntityPdu),
    /// Valid header, type this program does not decode.
    Other(PduHeader),
}

/// Parse one PDU; `buf` starts at its header.
pub fn parse_pdu(buf: &[u8]) -> Result<Pdu, ParseError> {
    let header = PduHeader::parse(buf)?;
    match header.pdu_type {
        pdu_type::ENTITY_STATE => EntityStatePdu::parse(buf).map(Pdu::EntityState),
        pdu_type::REMOVE_ENTITY => RemoveEntityPdu::parse(buf).map(Pdu::RemoveEntity),
        _ => Ok(Pdu::Other(header)),
    }
}

/// Split a buffer of back-to-back PDUs (a file, or a v7 bundled datagram)
/// using each header's length field. Stops at the first malformed header
/// and returns the offset reached alongside the slices found.
pub fn split_pdus(data: &[u8]) -> (Vec<(usize, &[u8])>, Result<(), (usize, String)>) {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let header = match PduHeader::parse(&data[offset..]) {
            Ok(h) => h,
            Err(e) => return (out, Err((offset, e.to_string()))),
        };
        let end = offset + header.length as usize;
        if end > data.len() {
            return (out, Err((offset, format!("PDU length {} runs past end of data", header.length))));
        }
        out.push((offset, &data[offset..end]));
        offset = end;
    }
    (out, Ok(()))
}

// ---------------------------------------------------------------------------
// Test encoder — builds PDUs field by field from the offsets above
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod encode {
    use super::*;

    pub fn header(pdu_type: u8, family: u8, exercise: u8, length: u16) -> Vec<u8> {
        let mut b = vec![7, exercise, pdu_type, family];
        b.extend_from_slice(&0x0000_1234u32.to_be_bytes()); // timestamp
        b.extend_from_slice(&length.to_be_bytes());
        b.extend_from_slice(&[0, 0]); // PDU status, padding
        b
    }

    pub struct EntityStateSpec<'a> {
        pub id: (u16, u16, u16),
        pub force: u8,
        pub entity_type: [u8; 8],
        pub velocity: [f32; 3],
        pub location: [f64; 3],
        pub orientation: [f32; 3],
        pub marking: &'a str,
        pub exercise: u8,
    }

    pub fn entity_state(s: &EntityStateSpec) -> Vec<u8> {
        let mut b = header(pdu_type::ENTITY_STATE, family::ENTITY_INFORMATION,
            s.exercise, ENTITY_STATE_MIN_LEN as u16);
        for v in [s.id.0, s.id.1, s.id.2] { b.extend_from_slice(&v.to_be_bytes()); }
        b.push(s.force);
        b.push(0); // variable parameter count
        b.extend_from_slice(&s.entity_type);
        b.extend_from_slice(&[0; 8]); // alternative entity type
        for v in s.velocity { b.extend_from_slice(&v.to_be_bytes()); }
        for v in s.location { b.extend_from_slice(&v.to_be_bytes()); }
        for v in s.orientation { b.extend_from_slice(&v.to_be_bytes()); }
        b.extend_from_slice(&0u32.to_be_bytes()); // appearance
        b.extend_from_slice(&[0; 40]); // dead reckoning
        b.push(1); // marking character set: ASCII
        let mut chars = [0u8; 11];
        chars[..s.marking.len()].copy_from_slice(s.marking.as_bytes());
        b.extend_from_slice(&chars);
        b.extend_from_slice(&0u32.to_be_bytes()); // capabilities
        assert_eq!(b.len(), ENTITY_STATE_MIN_LEN);
        b
    }

    pub fn remove_entity(receiving: (u16, u16, u16), exercise: u8) -> Vec<u8> {
        let mut b = header(pdu_type::REMOVE_ENTITY, family::SIMULATION_MANAGEMENT,
            exercise, REMOVE_ENTITY_LEN as u16);
        for v in [0u16, 0, 0, receiving.0, receiving.1, receiving.2] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        b.extend_from_slice(&42u32.to_be_bytes());
        assert_eq!(b.len(), REMOVE_ENTITY_LEN);
        b
    }
}

#[cfg(test)]
mod tests {
    use super::encode::*;
    use super::*;
    use crate::map::projection::geodetic_to_ecef;

    /// Hand-written v7 header, byte by byte, independent of the encoder.
    #[test]
    fn header_fields_are_big_endian_in_ieee_order() {
        let raw = [7u8, 3, 1, 1, 0x01, 0x02, 0x03, 0x04, 0x00, 0x90, 0x00, 0x00];
        let h = PduHeader::parse(&raw).unwrap();
        assert_eq!(h.protocol_version, 7);
        assert_eq!(h.exercise_id, 3);
        assert_eq!(h.pdu_type, 1);
        assert_eq!(h.protocol_family, 1);
        assert_eq!(h.timestamp, 0x0102_0304);
        assert_eq!(h.length, 144);
    }

    #[test]
    fn header_rejects_short_and_bad_length() {
        assert!(matches!(PduHeader::parse(&[0; 11]), Err(ParseError::Truncated { .. })));
        let mut raw = [0u8; 12];
        raw[9] = 11;
        assert_eq!(PduHeader::parse(&raw), Err(ParseError::InvalidLength(11)));
    }

    fn ottawa_helicopter() -> Vec<u8> {
        let [x, y, z] = geodetic_to_ecef(45.4215, -75.6972, 300.0);
        entity_state(&EntityStateSpec {
            id: (1, 2, 3),
            force: 2,
            entity_type: [1, 2, 0, 124, 20, 1, 0, 0], // platform, air, Canada
            velocity: [3.0, 4.0, 0.0],
            location: [x, y, z],
            orientation: [0.5, -0.25, 0.0],
            marking: "HELO-1",
            exercise: 1,
        })
    }

    #[test]
    fn entity_state_round_trip() {
        let pdu = EntityStatePdu::parse(&ottawa_helicopter()).unwrap();
        assert_eq!(pdu.entity_id, EntityId::new(1, 2, 3));
        assert_eq!(pdu.force_id, 2);
        assert_eq!(pdu.entity_type.kind, kind::PLATFORM);
        assert_eq!(pdu.entity_type.domain, domain::AIR);
        assert_eq!(pdu.entity_type.country, 124);
        assert_eq!(pdu.marking, "HELO-1");
        assert_eq!(pdu.speed(), 5.0);

        let (lat, lon, alt) = pdu.geodetic();
        assert!((lat - 45.4215).abs() < 1e-9, "lat {lat}");
        assert!((lon + 75.6972).abs() < 1e-9, "lon {lon}");
        assert!((alt - 300.0).abs() < 1e-3, "alt {alt}");

        let rec = pdu.to_record(std::time::Instant::now());
        assert_eq!(rec.status, Status::Hostile);
        assert_eq!(rec.station, Station::Air);
        assert_eq!(rec.name, "HELO-1");
    }

    #[test]
    fn entity_state_truncated() {
        let buf = ottawa_helicopter();
        assert!(matches!(EntityStatePdu::parse(&buf[..143]),
            Err(ParseError::Truncated { expected: 144, actual: 143 })));
    }

    #[test]
    fn entity_state_requires_variable_parameter_bytes() {
        let mut buf = ottawa_helicopter();
        buf[19] = 2; // claims 2 records but carries none
        assert!(matches!(EntityStatePdu::parse(&buf),
            Err(ParseError::Truncated { expected: 176, .. })));
    }

    #[test]
    fn heading_cardinal_points_at_equator() {
        use std::f64::consts::FRAC_PI_2;
        // At (0°, 0°): north is ECEF +Z, east is ECEF +Y.
        let north = heading_from_orientation(0.0, 0.0, 0.0, -FRAC_PI_2);
        assert!(north < 1e-6 || north > 360.0 - 1e-6, "north {north}");
        let east = heading_from_orientation(0.0, 0.0, FRAC_PI_2, 0.0);
        assert!((east - 90.0).abs() < 1e-6, "east {east}");
        let west = heading_from_orientation(0.0, 0.0, -FRAC_PI_2, 0.0);
        assert!((west - 270.0).abs() < 1e-6, "west {west}");
    }

    #[test]
    fn remove_entity_round_trip() {
        let pdu = RemoveEntityPdu::parse(&remove_entity((1, 2, 3), 1)).unwrap();
        assert_eq!(pdu.receiving, EntityId::new(1, 2, 3));
        assert_eq!(pdu.request_id, 42);
    }

    #[test]
    fn dispatch_and_split_bundle() {
        let mut data = ottawa_helicopter();
        data.extend(remove_entity((1, 2, 3), 1));
        data.extend(header(pdu_type::FIRE, family::WARFARE, 1, 12));
        let (pdus, end) = split_pdus(&data);
        assert!(end.is_ok());
        assert_eq!(pdus.len(), 3);
        assert!(matches!(parse_pdu(pdus[0].1), Ok(Pdu::EntityState(_))));
        assert!(matches!(parse_pdu(pdus[1].1), Ok(Pdu::RemoveEntity(_))));
        assert!(matches!(parse_pdu(pdus[2].1), Ok(Pdu::Other(h)) if h.pdu_type == pdu_type::FIRE));
    }

    #[test]
    fn split_reports_overrun() {
        let buf = ottawa_helicopter();
        let (pdus, end) = split_pdus(&buf[..100]);
        assert!(pdus.is_empty());
        assert_eq!(end.unwrap_err().0, 0);
    }
}
