//! Regression tests over the recorded DIS vectors in `tests/data/dis/`
//! (Sensors CGF recordings, copied — see that directory's README).

use super::packet::{parse_pdu, split_pdus, Pdu};
use std::path::PathBuf;

/// Canada in SISO-REF-010 (not ISO 3166's 124, which is Lesotho in DIS).
const CANADA: u16 = 39;

fn load(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/dis").join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Decode a whole recording; return (Entity State, Remove Entity) counts.
fn decode(name: &str) -> (usize, usize) {
    let data = load(name);
    let (pdus, tail) = split_pdus(&data);
    assert!(tail.is_ok(), "{name}: framing error {tail:?}");
    let (mut es, mut rm) = (0, 0);
    for (offset, bytes) in pdus {
        match parse_pdu(bytes).unwrap_or_else(|e| panic!("{name} @{offset}: {e:?}")) {
            Pdu::EntityState(p) => {
                assert_eq!(p.entity_type.country, CANADA, "{name} @{offset}");
                es += 1;
            }
            Pdu::RemoveEntity(_) => rm += 1,
            Pdu::Other(h) => panic!("{name} @{offset}: unexpected PDU type {}", h.pdu_type),
        }
    }
    (es, rm)
}

#[test]
fn lifecycle_decodes() { assert_eq!(decode("lifecycle.bin"), (237, 5)); }

#[test]
fn load_200_decodes() { assert_eq!(decode("load-200.bin"), (3020, 200)); }

#[test]
fn ottawa_mixed_decodes() { assert_eq!(decode("ottawa-mixed.bin"), (541, 13)); }

#[test]
fn zone_crossing_decodes() { assert_eq!(decode("zone-crossing.bin"), (541, 5)); }
