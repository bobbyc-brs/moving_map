/// Application state: entity table, map parameters, UI toggles.

use std::collections::HashMap;

use crate::dis::entity::{EntityId, EntityRecord, Status};
use crate::dis::packet::EntityDeathPdu;
use crate::dis::packet::EntityStatePdu;
use crate::map::projection::LatLon;

/// Application state shared between the DIS receiver and the renderer.

#[derive(Debug)]
pub struct AppState {
    /// All tracked entities.
    pub entities: HashMap<EntityId, EntityRecord>,
    /// Ownship entity ID (the first friendly entity seen, or manually set).
    pub ownship_id: Option<EntityId>,
    /// Ownship current position (cached for projection).
    pub ownship_latlon: Option<LatLon>,
    /// Display range in nautical miles.
    pub range_nm: f64,
    /// Current screen dimensions (updated on resize).
    pub screen_width: f64,
    pub screen_height: f64,
    /// Show range rings.
    pub show_range_rings: bool,
    /// Show grid.
    pub show_grid: bool,
    /// Filter: only show entities with this status (None = show all).
    pub status_filter: Option<Status>,
    /// Currently selected entity for info panel.
    pub selected_entity: Option<EntityId>,
    /// FPS counter (rolling average of last 60 frames).
    pub fps: f64,
    /// Discovered multicast address (for display).
    pub multicast_address: String,
}

impl AppState {
    pub fn new(range_nm: f64, screen_width: f64, screen_height: f64, multicast_address: String) -> Self {
        Self {
            entities: HashMap::new(),
            ownship_id: None,
            ownship_latlon: None,
            range_nm,
            screen_width,
            screen_height,
            show_range_rings: true,
            show_grid: true,
            status_filter: None,
            selected_entity: None,
            fps: 0.0,
            multicast_address,
        }
    }

    /// Process an incoming Entity State PDU.
    pub fn handle_entity_state(&mut self, pdu: EntityStatePdu) {
        let record = pdu.to_record(
            0, // default status (would come from the receiver's raw byte)
            pdu.marking,
            std::time::Instant::now(),
        );

        // Auto-select ownship: first friendly entity becomes ownship.
        if self.ownship_id.is_none() && record.status == Status::Friendly {
            self.ownship_id = Some(record.entity_id);
        }

        self.entities.insert(record.entity_id, record);
    }

    /// Process an incoming Entity Death PDU.
    pub fn handle_entity_death(&mut self, pdu: EntityDeathPdu) {
        self.entities.remove(&pdu.entity_id);
        if self.ownship_id == Some(pdu.entity_id) {
            self.ownship_id = None;
            self.ownship_latlon = None;
        }
        if self.selected_entity == Some(pdu.entity_id) {
            self.selected_entity = None;
        }
    }

    /// Remove all stale entities (not heard from within 15 seconds).
    pub fn purge_stale(&mut self) {
        let before = self.entities.len();
        self.entities.retain(|_, rec| !rec.is_stale(15.0));
        let purged = before - self.entities.len();
        if purged > 0 {
            log::debug!("Purged {} stale entities", purged);
        }
    }

    /// Update the ownship cached position.
    pub fn update_ownship_position(&mut self) {
        if let Some(id) = self.ownship_id {
            if let Some(rec) = self.entities.get(&id) {
                self.ownship_latlon = Some(LatLon {
                    lat: rec.latitude,
                    lon: rec.longitude,
                });
            }
        }
    }

    /// Iterate over entities, applying the current status filter.
    pub fn visible_entities(&self) -> impl Iterator<Item = &EntityRecord> {
        self.entities.values().filter(|e| {
            match self.status_filter {
                Some(status) => e.status == status,
                None => true,
            }
        })
    }

    /// Get the selected entity, if any.
    pub fn selected_record(&self) -> Option<&EntityRecord> {
        self.selected_entity.and_then(|id| self.entities.get(&id))
    }

    /// Number of entities currently tracked.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Number of visible entities (after filter).
    pub fn visible_count(&self) -> usize {
        self.visible_entities().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dis::entity::{Activity, EntityId, Station};

    fn make_record(id: EntityId, lat: f64, lon: f64) -> EntityRecord {
        EntityRecord::new(
            id,
            "TEST".to_string(),
            Station::Infantry,
            Activity::Squad,
            Status::Friendly,
            lat,
            lon,
            100.0,
            45.0,
            10.0,
            std::time::Instant::now(),
        )
    }

    #[test]
    fn insert_and_count() {
        let mut state = AppState::new(10.0, 1920.0, 1080.0, "test".into());
        let id1 = EntityId::new(1, 1, 1);
        let id2 = EntityId::new(1, 1, 2);

        state.entities.insert(make_record(id1, 45.0, -75.0).entity_id, make_record(id1, 45.0, -75.0));
        state.entities.insert(make_record(id2, 45.1, -75.1).entity_id, make_record(id2, 45.1, -75.1));

        assert_eq!(state.entity_count(), 2);
    }

    #[test]
    fn status_filter() {
        let mut state = AppState::new(10.0, 1920.0, 1080.0, "test".into());
        let id1 = EntityId::new(1, 1, 1);
        let id2 = EntityId::new(1, 1, 2);

        let mut r1 = make_record(id1, 45.0, -75.0);
        r1.status = Status::Friendly;
        let mut r2 = make_record(id2, 45.1, -75.1);
        r2.status = Status::Hostile;

        state.entities.insert(id1, r1);
        state.entities.insert(id2, r2);

        // No filter: both visible
        assert_eq!(state.visible_count(), 2);

        // Filter friendly: one visible
        state.status_filter = Some(Status::Friendly);
        assert_eq!(state.visible_count(), 1);

        // Filter hostile: one visible
        state.status_filter = Some(Status::Hostile);
        assert_eq!(state.visible_count(), 1);
    }
}
