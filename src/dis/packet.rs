/// DIS PDU packet parsing.
///
/// HLA DIS version 2, family 3 (Entity Simulation).

use crate::dis::entity::{Activity, EntityId, EntityRecord, Station, Status};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ParseError {
    Truncated { expected: usize, actual: usize },
    InvalidLength(u16),
    UnknownPduType(u8),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { expected, actual } => {
                write!(f, "PDU too short: expected {expected}, got {actual}")
            }
            Self::InvalidLength(len) => write!(f, "Invalid PDU length: {len}"),
            Self::UnknownPduType(t) => write!(f, "Unknown PDU type: {t}"),
        }
    }
}

impl std::error::Error for ParseError {}

// ---------------------------------------------------------------------------
// PDU header (12 bytes, big-endian)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PduHeader {
    pub pdu_type: u8,
    pub protocol_version: u8,
    pub exercise_id: u8,
    pub length: u16,
    pub protocol_family: u8,
    pub timestamp: u32,
}

impl PduHeader {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        if buf.len() < 12 {
            return Err(ParseError::Truncated {
                expected: 12,
                actual: buf.len(),
            });
        }
        let pdu_type = buf[0];
        let protocol_version = buf[2];
        let exercise_id = buf[3];
        let length = u16::from_be_bytes([buf[4], buf[5]]);
        let protocol_family = buf[6];
        let timestamp = u32::from_be_bytes([buf[7], buf[8], buf[9], buf[10]]);

        if length < 12 {
            return Err(ParseError::InvalidLength(length));
        }

        Ok(Self {
            pdu_type,
            protocol_version,
            exercise_id,
            length,
            protocol_family,
            timestamp,
        })
    }
}

// ---------------------------------------------------------------------------
// Entity State PDU (type 1, family 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntityStatePdu {
    pub header: PduHeader,
    pub entity_id: EntityId,
    pub marking: u16,
    pub entity_type: u8,
    pub altitude: f64,
    pub altitude2: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub linear_velocity_x: f32,
    pub linear_velocity_y: f32,
    pub linear_velocity_z: f32,
    pub angular_velocity_x: f32,
    pub angular_velocity_y: f32,
    pub angular_velocity_z: f32,
}

impl EntityStatePdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::parse(buf)?;
        if header.pdu_type != 1 {
            return Err(ParseError::UnknownPduType(header.pdu_type));
        }
        if header.protocol_family != 3 {
            return Err(ParseError::UnknownPduType(header.protocol_family));
        }
        if buf.len() < 105 {
            return Err(ParseError::Truncated {
                expected: 105,
                actual: buf.len(),
            });
        }

        let entity_id = EntityId::new(
            u16::from_le_bytes([buf[18], buf[19]]),
            u16::from_le_bytes([buf[20], buf[21]]),
            u16::from_le_bytes([buf[22], buf[23]]),
        );
        let marking = u16::from_le_bytes([buf[50], buf[51]]);
        let entity_type = buf[13];

        let lat_raw = i32::from_le_bytes([buf[38], buf[39], buf[40], buf[41]]);
        let latitude = lat_raw as f64 / 10_000_000.0;
        let lon_raw = i32::from_le_bytes([buf[42], buf[43], buf[44], buf[45]]);
        let longitude = lon_raw as f64 / 10_000_000.0;
        let alt_raw = i32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
        let altitude = alt_raw as f64 / 1000.0;
        let alt2_raw = i32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        let altitude2 = alt2_raw as f64 / 1000.0;

        let vel_x = i32::from_le_bytes([buf[69], buf[70], buf[71], buf[72]]) as f32;
        let vel_y = i32::from_le_bytes([buf[73], buf[74], buf[75], buf[76]]) as f32;
        let vel_z = i32::from_le_bytes([buf[77], buf[78], buf[79], buf[80]]) as f32;

        let ang_x = i32::from_le_bytes([buf[81], buf[82], buf[83], buf[84]]) as f32;
        let ang_y = i32::from_le_bytes([buf[85], buf[86], buf[87], buf[88]]) as f32;
        let ang_z = i32::from_le_bytes([buf[89], buf[90], buf[91], buf[92]]) as f32;

        Ok(Self {
            header, entity_id, marking, entity_type,
            altitude, altitude2, latitude, longitude,
            linear_velocity_x: vel_x, linear_velocity_y: vel_y, linear_velocity_z: vel_z,
            angular_velocity_x: ang_x, angular_velocity_y: ang_y, angular_velocity_z: ang_z,
        })
    }

    pub fn velocity_magnitude(&self) -> f32 {
        (self.linear_velocity_x.powi(2)
            + self.linear_velocity_y.powi(2)
            + self.linear_velocity_z.powi(2)).sqrt()
    }

    pub fn heading_degrees(&self) -> f32 {
        let h = self.linear_velocity_y.atan2(self.linear_velocity_x).to_degrees();
        if h < 0.0 { h + 360.0 } else { h }
    }

    pub fn station(&self) -> Station {
        Station::from_station_id(self.entity_type & 0x0F)
    }

    pub fn activity(&self) -> Activity {
        let domain = (self.entity_type >> 4) & 0x0F;
        Activity::from_activity_id(domain as u8, self.entity_type & 0x0F)
    }

    pub fn to_record(
        self,
        status_byte: u8,
        last_heard: std::time::Instant,
    ) -> EntityRecord {
        EntityRecord::new(
            self.entity_id,
            format!("M{:04X}", self.marking),
            self.station(),
            self.activity(),
            Status::from_discriminator(status_byte),
            self.latitude,
            self.longitude,
            self.altitude,
            self.heading_degrees(),
            self.velocity_magnitude(),
            last_heard,
        )
    }
}

// ---------------------------------------------------------------------------
// Entity Death PDU (type 2, family 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct EntityDeathPdu {
    pub header: PduHeader,
    pub entity_id: EntityId,
    pub cause: u8,
    pub area_or_object_id: u16,
}

impl EntityDeathPdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::parse(buf)?;
        if header.pdu_type != 2 {
            return Err(ParseError::UnknownPduType(header.pdu_type));
        }
        if buf.len() < 24 {
            return Err(ParseError::Truncated {
                expected: 24,
                actual: buf.len(),
            });
        }
        let entity_id = EntityId::new(
            u16::from_le_bytes([buf[18], buf[19]]),
            u16::from_le_bytes([buf[20], buf[21]]),
            u16::from_le_bytes([buf[22], buf[23]]),
        );
        Ok(Self {
            header, entity_id,
            cause: buf[24],
            area_or_object_id: u16::from_le_bytes([buf[25], buf[26]]),
        })
    }
}

// ---------------------------------------------------------------------------
// Entity Recreation PDU (type 4, family 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntityRecreationPdu {
    pub header: PduHeader,
    pub response_number: u16,
    pub site: u16,
    pub application: u16,
    pub entity: u16,
    pub entity_type: u8,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

impl EntityRecreationPdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::parse(buf)?;
        if header.pdu_type != 4 {
            return Err(ParseError::UnknownPduType(header.pdu_type));
        }
        if buf.len() < 52 {
            return Err(ParseError::Truncated {
                expected: 52,
                actual: buf.len(),
            });
        }
        Ok(Self {
            header,
            response_number: u16::from_le_bytes([buf[12], buf[13]]),
            site: u16::from_le_bytes([buf[14], buf[15]]),
            application: u16::from_le_bytes([buf[16], buf[17]]),
            entity: u16::from_le_bytes([buf[18], buf[19]]),
            entity_type: buf[20],
            latitude: {
                let raw = i32::from_le_bytes([buf[38], buf[39], buf[40], buf[41]]);
                raw as f64 / 10_000_000.0
            },
            longitude: {
                let raw = i32::from_le_bytes([buf[42], buf[43], buf[44], buf[45]]);
                raw as f64 / 10_000_000.0
            },
            altitude: {
                let raw = i32::from_le_bytes([buf[46], buf[47], buf[48], buf[49]]);
                raw as f64 / 1000.0
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Entity Detection PDU (type 5, family 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntityDetectionPdu {
    pub header: PduHeader,
    pub responding_entity: EntityId,
    pub detected_entity: EntityId,
    pub sensor_id: u16,
    pub detection_type: u8,
    pub relative_position_x: f32,
    pub relative_position_y: f32,
    pub relative_position_z: f32,
    pub detection_probability: f32,
}

impl EntityDetectionPdu {
    pub fn parse(buf: &[u8]) -> Result<Self, ParseError> {
        let header = PduHeader::parse(buf)?;
        if header.pdu_type != 5 {
            return Err(ParseError::UnknownPduType(header.pdu_type));
        }
        if buf.len() < 50 {
            return Err(ParseError::Truncated {
                expected: 50,
                actual: buf.len(),
            });
        }
        Ok(Self {
            header,
            responding_entity: EntityId::new(
                u16::from_le_bytes([buf[14], buf[15]]),
                u16::from_le_bytes([buf[16], buf[17]]),
                u16::from_le_bytes([buf[18], buf[19]]),
            ),
            detected_entity: EntityId::new(
                u16::from_le_bytes([buf[20], buf[21]]),
                u16::from_le_bytes([buf[22], buf[23]]),
                u16::from_le_bytes([buf[24], buf[25]]),
            ),
            sensor_id: u16::from_le_bytes([buf[26], buf[27]]),
            detection_type: buf[28],
            relative_position_x: i32::from_le_bytes([buf[29], buf[30], buf[31], buf[32]]) as f32,
            relative_position_y: i32::from_le_bytes([buf[33], buf[34], buf[35], buf[36]]) as f32,
            relative_position_z: i32::from_le_bytes([buf[37], buf[38], buf[39], buf[40]]) as f32,
            detection_probability: buf[41] as f32 / 255.0,
        })
    }
}
