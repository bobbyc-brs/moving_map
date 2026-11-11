---
date: 2026-11-08
updated: 2026-11-10
---

# Test Plan — moving-map-display

## Structure

Unit tests live alongside source in `src/` modules. Integration tests
(symbology rendering, end-to-end DIS flow) live in `tests/`.

```
src/
  dis/
    packet.rs          #[cfg(test)] unit tests
    receiver.rs        #[cfg(test)] loopback UDP socket tests
  map/
    projection.rs      #[cfg(test)] coordinate transforms
    mgrs.rs            #[cfg(test)] grid generation
  render/
    nato.rs            #[cfg(test)] symbol geometry
app/
  state.rs             #[cfg(test)] entity lifecycle — NOT COMPILED (PLAN.md Discrepancies #8)
tests/
  integration/
    dis_roundtrip.rs   — simulate DIS PDUs, verify entity table
    render_frame.rs    — render frame, verify entity count on screen
```

## Unit tests

### DIS packet parsing

*Revised 2026-11-10 (DD-004): the original rows tested Death / Recreation PDUs
that are not IEEE 1278.1, at sizes that do not match the standard. Implemented
tests named in brackets.*

| Test | Input | Expected |
|---|---|---|
| `parse_entity_state_valid` [`entity_state_round_trip`] | 144-byte v7 Entity State | ID, force, type, marking, ECEF → lat/lon/alt exact |
| `parse_entity_state_short` [`entity_state_truncated`] | 143 bytes | `Err(Truncated { expected: 144 })` |
| `parse_entity_state_var_params` [`entity_state_requires_variable_parameter_bytes`] | count = 2, no records | `Err(Truncated { expected: 176 })` |
| `parse_remove_entity_valid` [`remove_entity_round_trip`] | 28-byte Remove Entity | receiving ID, request ID |
| `parse_header_fields` [`header_fields_are_big_endian_in_ieee_order`] | hand-written bytes | IEEE field order, big-endian |
| `parse_header_bad_length` [`header_rejects_short_and_bad_length`] | length field 11 | `Err(InvalidLength(11))` |
| `parse_unknown_type` [`dispatch_and_split_bundle`] | Fire PDU (type 2) | `Pdu::Other`, not an error |
| `split_bundle` [`dispatch_and_split_bundle`] | 3 PDUs in one buffer | 3 slices |

### DIS entity table

| Test | Action | Expected |
|---|---|---|
| `insert_new_entity` | Insert EntityState for entity ID 42 | `entities.len() == 1` |
| `update_existing_entity` | Insert, then update same ID with new position | Position updated, timestamp refreshed |
| `delete_entity` | Send Remove Entity PDU for entity ID 42 | Entity removed from table |
| `stale_entities` | Insert entity, advance clock 13 s | `is_stale()` returns true (12 s timeout, DD-005) |
| `entity_not_stale` | Insert entity, advance clock 5 s | `is_stale()` returns false |
| `reappear_after_remove` | Remove Entity, then Entity State with same ID | Entity re-added *(revised 2026-11-10: was "recreation PDU")* |

### Map projection

*Revised 2026-11-10 (DD-006): tolerance set to **10 cm** (Bobby); the original
rows named wrong zones (45°N 75.6°W is zone 18, not 17; 72°W starts zone 19).*

| Test | Input | Expected |
|---|---|---|
| `utm_round_trip_within_10_cm` | 45 points, 70°S–70°N, zone centre to edges | worst round trip < 10 cm (measured < 0.1 mm) |
| `central_meridian_matches_meridian_arc` | 75°W at 10/45/45.42/70°N | easting 500 000 m; northing = k0 × numeric meridian arc ± 1 mm |
| `equator_zone_edge_easting` | (0°, 0°) | zone 31, easting 166 021.44 m |
| `southern_hemisphere_false_northing` | just south of the equator | northing ≈ 10 000 000 m |
| `zone_boundaries` | 72.000001°W / 72°W / ±180° | zones 18 / 19 / 60 / 1 |
| `ecef_round_trip_sub_millimetre` | 4 points incl. 89.9°N | lat/lon 1e-8°, height 1 mm |
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
4. Stop sender, wait 13 s (12 s timeout, DD-005):
   - Verify entity is marked stale.
5. Send Remove Entity PDU *(revised 2026-11-10: was "Entity Death")*:
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
