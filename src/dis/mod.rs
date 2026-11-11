/// DIS (Distributed Interactive Simulation) protocol layer.

pub mod entity;
pub mod packet;
pub mod receiver;

use std::collections::HashMap;
use std::time::Instant;

use crate::dis::entity::{EntityId, EntityRecord};
use crate::dis::packet::Pdu;

/// Where to listen for DIS on the LAN. Traffic is local-network UDP only;
/// nothing here reaches beyond the LAN.
#[derive(Debug, Clone)]
pub struct DisConfig {
    /// UDP port. 3000 is the conventional DIS port — confirm against the
    /// simulators on site (PLAN.md ⚖).
    pub port: u16,
    /// Multicast group to join; `None` receives broadcast and unicast only.
    pub multicast_group: Option<std::net::Ipv4Addr>,
    /// Local interface address for the multicast join; `None` lets the OS pick.
    pub interface: Option<std::net::Ipv4Addr>,
    /// Accept only this exercise ID; `None` accepts all.
    pub exercise_id: Option<u8>,
}

impl Default for DisConfig {
    fn default() -> Self {
        Self { port: 3000, multicast_group: None, interface: None, exercise_id: None }
    }
}

/// Seconds without an Entity State PDU before an entity is dropped:
/// the IEEE 1278.1 default heartbeat (5 s) × timeout multiplier (2.4).
pub const ENTITY_TIMEOUT_SECS: f64 = 12.0;

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

    /// Apply one parsed PDU. Returns true if the table changed.
    pub fn apply(&mut self, pdu: &Pdu, now: Instant) -> bool {
        match pdu {
            Pdu::EntityState(es) => {
                self.upsert(es.to_record(now));
                true
            }
            // The receiving entity ID names the entity to remove.
            Pdu::RemoveEntity(re) => self.entities.remove(&re.receiving).is_some(),
            Pdu::Other(_) => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dis::packet::{encode, parse_pdu};
    use crate::map::projection::geodetic_to_ecef;

    #[test]
    fn apply_state_then_remove() {
        let [x, y, z] = geodetic_to_ecef(45.0, -75.0, 0.0);
        let es = encode::entity_state(&encode::EntityStateSpec {
            id: (5, 6, 7), force: 1, entity_type: [1, 1, 0, 39, 1, 0, 0, 0],
            velocity: [0.0; 3], location: [x, y, z], orientation: [0.0; 3],
            marking: "TANK", exercise: 1,
        });
        let mut table = EntityTable::new();
        assert!(table.apply(&parse_pdu(&es).unwrap(), Instant::now()));
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(EntityId::new(5, 6, 7)).unwrap().name, "TANK");

        let rm = encode::remove_entity((5, 6, 7), 1);
        assert!(table.apply(&parse_pdu(&rm).unwrap(), Instant::now()));
        assert!(table.is_empty());
    }
}
