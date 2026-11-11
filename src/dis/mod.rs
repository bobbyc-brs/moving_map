/// DIS (Distributed Interactive Simulation) protocol layer.

pub mod entity;
pub mod packet;

use std::collections::HashMap;

use crate::dis::entity::{EntityId, EntityRecord, Station, Status};
use crate::dis::packet::EntityRecreationPdu;

#[derive(Debug, Clone)]
pub struct DisConfig {
    pub multicast_address: String,
    pub port: u16,
    pub interface: String,
}

impl Default for DisConfig {
    fn default() -> Self {
        Self {
            multicast_address: "239.255.1.1".to_string(),
            port: 30000,
            interface: String::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct EntityTable {
    entities: HashMap<EntityId, EntityRecord>,
}

impl EntityTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, record: EntityRecord) {
        self.entities.insert(record.entity_id, record);
    }

    pub fn remove(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    pub fn handle_recreation(&mut self, pdu: EntityRecreationPdu) {
        let id = EntityId::new(pdu.site, pdu.application, pdu.entity);
        self.upsert(EntityRecord::new(
            id,
            format!("R:{}", id.display()),
            Station::CommandControl,
            entity::Activity::Unknown,
            Status::Unknown,
            pdu.latitude,
            pdu.longitude,
            pdu.altitude,
            0.0,
            0.0,
            std::time::Instant::now(),
        ));
    }

    pub fn purge_stale(&mut self, timeout_secs: f64) -> usize {
        let before = self.entities.len();
        self.entities.retain(|_, rec| !rec.is_stale(timeout_secs));
        before - self.entities.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntityRecord> {
        self.entities.values()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn get(&self, id: EntityId) -> Option<&EntityRecord> {
        self.entities.get(&id)
    }
}
