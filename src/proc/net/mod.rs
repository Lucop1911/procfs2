/// ARP table from `/proc/net/arp`.
pub mod arp;
/// Per-interface network statistics from `/proc/net/dev`.
pub mod dev;
/// IP routing table from `/proc/net/route`.
pub mod route;
/// IPv4 protocol counters from `/proc/net/snmp`.
pub mod snmp;
/// IPv6 protocol counters from `/proc/net/snmp6`.
pub mod snmp6;
/// TCP connections from `/proc/net/tcp` and `/proc/net/tcp6`.
pub mod tcp;
/// UDP sockets from `/proc/net/udp` and `/proc/net/udp6`.
pub mod udp;
/// Unix domain sockets from `/proc/net/unix`.
pub mod unix;

pub use arp::{ArpEntry, arp};
pub use dev::{NetDevStat, dev};
pub use route::{RouteEntry, route};
pub use snmp::{IcmpStats, IpStats, Snmp, SnmpInfo, TcpStats, UdpStats, snmp};
pub use snmp6::{Icmp6Stats, Ip6Stats, Snmp6Info, Udp6Stats, snmp6};
pub use tcp::{TcpEntry, TcpState, tcp, tcp6};
pub use udp::{Udp6Entry, UdpEntry, udp, udp6};
pub use unix::{UnixEntry, unix};

/// Hex length of an IPv4 address in `/proc/net` tables (`0100007F`).
pub(crate) const IPV4_HEX_LEN: usize = 8;
/// Hex length of an IPv6 address in `/proc/net` tables (four 32-bit words).
pub(crate) const IPV6_HEX_LEN: usize = 32;

/// Columns in a `/proc/net/{tcp,tcp6,udp,udp6}` row, header excluded.
pub(crate) const SOCKET_TABLE_FIELDS: usize = 12;
// Column indices shared by those tables, in the order the header prints them.
pub(crate) const COL_LOCAL: usize = 1;
pub(crate) const COL_REMOTE: usize = 2;
pub(crate) const COL_STATE: usize = 3;
pub(crate) const COL_QUEUE: usize = 4;
pub(crate) const COL_UID: usize = 7;
pub(crate) const COL_INODE: usize = 9;
