---
date: 2026-11-08
---

# Test Plan — moving-map-display

## Structure

Unit tests live alongside source in `src/` modules. Integration tests
(symbology rendering, end-to-end DIS flow) live in `tests/`.

```
src/
  dis/
    packet.rs          #[cfg(test)] unit tests
    receiver.rs        #[cfg(test)] mocked socket tests
  map/
    projection.rs      #[cfg(test)] coordinate transforms
    mgrs.rs            #[cfg(test)] grid generation
  render/
    nato.rs            #[cfg(test)] symbol geometry
app/
  state.rs             #[cfg(test)] entity lifecycle
tests/
  integration/
    dis_roundtrip.rs   — simulate DIS PDUs, verify entity table
    render_frame.rs    — render frame, verify entity count on screen
```

## Unit tests

### DIS packet parsing

| Test | Input | Expected |
|---|---|---|
| `parse_entity_state_valid` | 146-byte valid PDU blob | `EntityStatePdu` with correct fields |
| `parse_entity_state_short` | 100-byte PDU blob (truncated) | `Err(ParseError::Truncated)` |
| `parse_entity_death_valid` | 12-byte death PDU | `EntityDeathPdu` with correct IDs |
| `parse_recreation_valid` | 40-byte recreation PDU | `EntityRecreationPdu` with correct template |
| `parse_header_unknown_type` | Header with type 99 | `Err(ParseError::UnknownPduType(99))` |
| `parse_header_bad_length` | Header with length < 12 | `Err(ParseError::InvalidLength(9))` |

### DIS entity table

| Test | Action | Expected |
|---|---|---|
| `insert_new_entity` | Insert EntityState for entity ID 42 | `entities.len() == 1` |
| `update_existing_entity` | Insert, then update same ID with new position | Position updated, timestamp refreshed |
| `delete_entity` | Send death PDU for entity ID 42 | Entity removed from table |
| `stale_entities` | Insert entity, advance clock 16 s | `is_stale()` returns true |
| `entity_not_stale` | Insert entity, advance clock 5 s | `is_stale()` returns false |
| `recreate_after_death` | Death then recreation with same ID | Entity re-added with template data |

### Map projection

| Test | Input | Expected |
|---|---|---|
| `ll_to_utm_wgs84` | LatLon(45.0, -75.6) | Correct UTM zone 17U, easting/northing |
| `utm_to_ll_roundtrip` | UTM(500000, 5000000) zone 17U | LatLon within 1 mm of original |
| `zone_boundary` | LatLon(44.9, -72.0) near zone 18 | Correct zone 18 |
| `centre_offset_default` | Any world coord | Ownship maps to 50%x, 70%y screen |
| `centre_offset_custom` | Ownship at 50%x, 80%y | Configurable |

### NATO symbology

| Test | Station/Status | Expected shape |
|---|---|---|
| `friendly_infantry` | Friendly + Infantry | Diamond shape, blue fill |
| `hostile_tank` | Hostile + Surface + Tank | Rectangle, red fill, X modifier |
| `friendly_aircraft` | Friendly + Air + Fighter | Rectangle, blue fill, arrow wings |
| `unknown_surface` | Unknown + Surface | Diamond, yellow fill |
| `neutral_ship` | Neutral + SurfaceVessel | Rectangle, green fill, wave |
| `ownship_symbol` | Ownship | Aircraft outline + heading vector |

### Entity info panel

| Test | Input | Expected |
|---|---|---|
| `display_friendly_entity` | Click friendly entity | Info panel shows all DIS fields |
| `display_stale_entity` | Click stale entity | Warning icon, "Last heard: N s ago" |
| `panel_closed` | Click entity info close | Panel hidden |
| `no_selection` | Click empty space | Panel hidden or "no entity" |

## Integration tests

### DIS round-trip (mocked)

1. Create `dis::receiver` bound to a test multicast group.
2. Spawn a UDP sender that transmits a known Entity State PDU every 2 s.
3. Run receiver for 10 s, then:
   - Verify entity table contains the sender.
   - Verify position matches the sent PDU.
   - Verify last\_heard is recent.
4. Stop sender, wait 16 s:
   - Verify entity is marked stale.
5. Send Entity Death PDU:
   - Verify entity removed.

### Render frame (headless)

1. Create app state with 20 entities at known positions.
2. Render one frame at 1920×1080.
3. Capture the egui output as a texture or raster.
4. Verify:
   - Ownship symbol is visible.
   - All 20 entities have symbols within the viewport.
   - Grid lines are present and labelled.
   - Range rings are present and labelled.
   - Scale bar is present.

### Stress: entity density

1. Seed entity table with 200 entities randomly distributed in a 100 NM radius.
2. Render 100 frames.
3. Verify:
   - Average FPS ≥ 30 (measured via `std::time::Instant`).
   - No panic or allocation failure.
   - Memory stays < 200 MB (tracked via RSS or allocator stats).

## Acceptance criteria

| # | Criterion | How verified |
|---|---|---|
| AC-001 | All unit tests pass (`cargo test`) | Automated |
| AC-002 | Integration tests pass (`cargo test --test`) | Automated |
| AC-003 | Entity State PDU from known simulator produces correct symbol | Manual + TESTLOG entry |
| AC-004 | 1080p display at ≥ 30 fps with 200 entities | Stress test + manual timing |
| AC-005 | NATO symbols match APP-6A specification | Visual comparison table |
