/// DIS UDP receiver — `std::net` only, for traffic on the local LAN.
///
/// Receives broadcast or unicast datagrams on the configured port, and
/// optionally joins one IPv4 multicast group. Each datagram may carry several
/// back-to-back PDUs (v7 bundling); each is parsed and filtered by exercise ID.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use crate::dis::packet::{parse_pdu, split_pdus, Pdu};
use crate::dis::DisConfig;

/// Largest datagram accepted. A DIS PDU's length field is a u16, so 64 KiB
/// covers any PDU or bundle.
pub const MAX_DATAGRAM: usize = 65_536;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Stats {
    pub datagrams: u64,
    pub pdus: u64,
    pub malformed: u64,
    pub other_exercise: u64,
}

pub struct Receiver {
    socket: UdpSocket,
    exercise_id: Option<u8>,
    buf: Vec<u8>,
    pub stats: Stats,
}

impl Receiver {
    /// Bind `0.0.0.0:<port>` and join the multicast group if one is set.
    pub fn bind(cfg: &DisConfig) -> io::Result<Self> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, cfg.port))?;
        Self::from_socket(socket, cfg)
    }

    /// Wrap an already-bound socket (tests bind to an ephemeral loopback port).
    pub fn from_socket(socket: UdpSocket, cfg: &DisConfig) -> io::Result<Self> {
        if let Some(group) = cfg.multicast_group {
            socket.join_multicast_v4(&group, &cfg.interface.unwrap_or(Ipv4Addr::UNSPECIFIED))?;
        }
        Ok(Self { socket, exercise_id: cfg.exercise_id, buf: vec![0; MAX_DATAGRAM], stats: Stats::default() })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Wait up to `timeout` for one datagram and return the PDUs it carried.
    /// A timeout returns an empty list, not an error.
    pub fn poll(&mut self, timeout: Duration) -> io::Result<Vec<Pdu>> {
        self.socket.set_read_timeout(Some(timeout.max(Duration::from_millis(1))))?;
        let n = match self.socket.recv_from(&mut self.buf) {
            Ok((n, _from)) => n,
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(e),
        };
        self.stats.datagrams += 1;

        let (slices, end) = split_pdus(&self.buf[..n]);
        if end.is_err() {
            self.stats.malformed += 1;
        }
        let mut out = Vec::with_capacity(slices.len());
        for (_, bytes) in slices {
            match parse_pdu(bytes) {
                Ok(pdu) => {
                    let exercise = match &pdu {
                        Pdu::EntityState(p) => p.header.exercise_id,
                        Pdu::RemoveEntity(p) => p.header.exercise_id,
                        Pdu::Other(h) => h.exercise_id,
                    };
                    if self.exercise_id.is_some_and(|want| want != exercise) {
                        self.stats.other_exercise += 1;
                        continue;
                    }
                    self.stats.pdus += 1;
                    out.push(pdu);
                }
                Err(_) => self.stats.malformed += 1,
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dis::packet::encode;
    use crate::map::projection::geodetic_to_ecef;

    fn loopback_receiver(cfg: &DisConfig) -> (Receiver, UdpSocket) {
        let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = Receiver::from_socket(rx, cfg).unwrap();
        let tx = UdpSocket::bind("127.0.0.1:0").unwrap();
        tx.connect(receiver.local_addr().unwrap()).unwrap();
        (receiver, tx)
    }

    fn state(exercise: u8) -> Vec<u8> {
        let [x, y, z] = geodetic_to_ecef(45.0, -75.0, 100.0);
        encode::entity_state(&encode::EntityStateSpec {
            id: (1, 1, 9), force: 1, entity_type: [1, 2, 0, 124, 0, 0, 0, 0],
            velocity: [0.0; 3], location: [x, y, z], orientation: [0.0; 3],
            marking: "UDP", exercise,
        })
    }

    #[test]
    fn receives_bundled_datagram_over_loopback() {
        let (mut rx, tx) = loopback_receiver(&DisConfig::default());
        let mut bundle = state(1);
        bundle.extend(encode::remove_entity((1, 1, 9), 1));
        tx.send(&bundle).unwrap();

        let pdus = rx.poll(Duration::from_secs(2)).unwrap();
        assert_eq!(pdus.len(), 2);
        assert!(matches!(&pdus[0], Pdu::EntityState(p) if p.marking == "UDP"));
        assert!(matches!(&pdus[1], Pdu::RemoveEntity(_)));
        assert_eq!(rx.stats, Stats { datagrams: 1, pdus: 2, malformed: 0, other_exercise: 0 });
    }

    #[test]
    fn filters_exercise_and_counts_malformed() {
        let cfg = DisConfig { exercise_id: Some(7), ..DisConfig::default() };
        let (mut rx, tx) = loopback_receiver(&cfg);
        tx.send(&state(1)).unwrap();
        assert!(rx.poll(Duration::from_secs(2)).unwrap().is_empty());
        tx.send(&state(7)[..50]).unwrap(); // header claims 144, only 50 sent
        assert!(rx.poll(Duration::from_secs(2)).unwrap().is_empty());
        assert_eq!(rx.stats.other_exercise, 1);
        assert_eq!(rx.stats.malformed, 1);
    }

    #[test]
    fn timeout_is_not_an_error() {
        let (mut rx, _tx) = loopback_receiver(&DisConfig::default());
        assert!(rx.poll(Duration::from_millis(20)).unwrap().is_empty());
    }
}
