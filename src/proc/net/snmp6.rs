use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// IPv6 packet statistics read from `/proc/net/snmp6`.
#[derive(Debug, Default)]
pub struct Ip6Stats {
    /// IPv6 packets received from interfaces.
    pub in_receives: u64,
    /// Packets discarded because of IPv6 header errors.
    pub in_hdr_errors: u64,
    /// Packets discarded because they exceeded the IPv6 packet size limit.
    pub in_too_big_errors: u64,
    /// Packets discarded because no route was available.
    pub in_no_routes: u64,
    /// Packets discarded because of invalid destination addresses.
    pub in_addr_errors: u64,
    /// Packets discarded because their protocol was unknown.
    pub in_unknown_protos: u64,
    /// Packets truncated before processing.
    pub in_truncated_pkts: u64,
    /// Incoming packets discarded for other reasons.
    pub in_discards: u64,
    /// Incoming packets delivered to higher-level protocols.
    pub in_delivers: u64,
    /// Forwarded packets sent.
    pub out_forw_datagrams: u64,
    /// Packets submitted by local protocols for transmission.
    pub out_requests: u64,
    /// Outgoing packets discarded for other reasons.
    pub out_discards: u64,
    /// Outgoing packets discarded because no route was available.
    pub out_no_routes: u64,
    /// Maximum fragment reassembly time.
    pub reasm_timeout: u64,
    /// Fragments received that required reassembly.
    pub reasm_reqds: u64,
    /// Datagrams successfully reassembled.
    pub reasm_oks: u64,
    /// Fragment reassembly failures.
    pub reasm_fails: u64,
    /// Datagrams successfully fragmented.
    pub frag_oks: u64,
    /// Datagrams that could not be fragmented.
    pub frag_fails: u64,
    /// Fragments created by IPv6 fragmentation.
    pub frag_creates: u64,
    /// Incoming multicast packets.
    pub in_mcast_pkts: u64,
    /// Outgoing multicast packets.
    pub out_mcast_pkts: u64,
    /// Bytes received.
    pub in_octets: u64,
    /// Bytes transmitted.
    pub out_octets: u64,
    /// Bytes received in multicast packets.
    pub in_mcast_octets: u64,
    /// Bytes transmitted in multicast packets.
    pub out_mcast_octets: u64,
    /// Bytes received in broadcast packets.
    pub in_bcast_octets: u64,
    /// Bytes transmitted in broadcast packets.
    pub out_bcast_octets: u64,
    /// Incoming packets with no ECN capability.
    pub in_no_ect_pkts: u64,
    /// Incoming packets marked with ECN ECT(1).
    pub in_ect1_pkts: u64,
    /// Incoming packets marked with ECN ECT(0).
    pub in_ect0_pkts: u64,
    /// Incoming packets marked with ECN-CE.
    pub in_ce_pkts: u64,
    /// Total IPv6 packets transmitted.
    pub out_transmits: u64,
}

/// ICMPv6 message statistics read from `/proc/net/snmp6`.
#[derive(Debug, Default)]
pub struct Icmp6Stats {
    /// ICMPv6 messages received.
    pub in_msgs: u64,
    /// ICMPv6 messages received with errors.
    pub in_errors: u64,
    /// ICMPv6 messages sent.
    pub out_msgs: u64,
    /// ICMPv6 messages not sent because of errors.
    pub out_errors: u64,
    /// ICMPv6 checksum errors received.
    pub in_csum_errors: u64,
    /// ICMPv6 messages limited by the host rate limiter.
    pub out_rate_limit_host: u64,
    /// Destination-unreachable messages received.
    pub in_dest_unreachs: u64,
    /// Packet-too-big messages received.
    pub in_pkt_too_bigs: u64,
    /// Time-exceeded messages received.
    pub in_time_excds: u64,
    /// Parameter-problem messages received.
    pub in_parm_problems: u64,
    /// Echo requests received.
    pub in_echos: u64,
    /// Echo replies received.
    pub in_echo_replies: u64,
    /// Group membership queries received.
    pub in_group_memb_queries: u64,
    /// Group membership responses received.
    pub in_group_memb_responses: u64,
    /// Group membership reductions received.
    pub in_group_memb_reductions: u64,
    /// Router solicitations received.
    pub in_router_solicits: u64,
    /// Router advertisements received.
    pub in_router_advertisements: u64,
    /// Neighbor solicitations received.
    pub in_neighbor_solicits: u64,
    /// Neighbor advertisements received.
    pub in_neighbor_advertisements: u64,
    /// Redirects received.
    pub in_redirects: u64,
    /// MLDv2 reports received.
    pub in_mldv2_reports: u64,
    /// Destination-unreachable messages sent.
    pub out_dest_unreachs: u64,
    /// Packet-too-big messages sent.
    pub out_pkt_too_bigs: u64,
    /// Time-exceeded messages sent.
    pub out_time_excds: u64,
    /// Parameter-problem messages sent.
    pub out_parm_problems: u64,
    /// Echo requests sent.
    pub out_echos: u64,
    /// Echo replies sent.
    pub out_echo_replies: u64,
    /// Group membership queries sent.
    pub out_group_memb_queries: u64,
    /// Group membership responses sent.
    pub out_group_memb_responses: u64,
    /// Group membership reductions sent.
    pub out_group_memb_reductions: u64,
    /// Router solicitations sent.
    pub out_router_solicits: u64,
    /// Router advertisements sent.
    pub out_router_advertisements: u64,
    /// Neighbor solicitations sent.
    pub out_neighbor_solicits: u64,
    /// Neighbor advertisements sent.
    pub out_neighbor_advertisements: u64,
    /// Redirects sent.
    pub out_redirects: u64,
    /// MLDv2 reports sent.
    pub out_mldv2_reports: u64,
}

/// UDP over IPv6 statistics read from `/proc/net/snmp6`.
#[derive(Debug, Default)]
pub struct Udp6Stats {
    /// UDPv6 datagrams delivered to applications.
    pub in_datagrams: u64,
    /// UDPv6 datagrams received for an unused port.
    pub no_ports: u64,
    /// UDPv6 receive errors other than unused ports.
    pub in_errors: u64,
    /// UDPv6 datagrams sent.
    pub out_datagrams: u64,
    /// Receive buffer errors.
    pub rcvbuf_errors: u64,
    /// Send buffer errors.
    pub sndbuf_errors: u64,
    /// UDPv6 checksum errors received.
    pub in_csum_errors: u64,
    /// Multicast datagrams ignored.
    pub ignored_multi: u64,
    /// UDPv6 memory allocation errors.
    pub mem_errors: u64,
}

/// IPv6 protocol statistics read from `/proc/net/snmp6`.
#[derive(Debug, Default)]
pub struct Snmp6Info {
    /// IPv6 packet counters.
    pub ip: Ip6Stats,
    /// ICMPv6 message counters.
    pub icmp: Icmp6Stats,
    /// UDPv6 datagram counters.
    pub udp: Udp6Stats,
}

/// Reads IPv6 protocol statistics from `/proc/net/snmp6`.
///
/// Each line contains one counter name and value. Unknown counters are
/// ignored so newer kernels can extend the file without breaking parsing.
pub fn snmp6() -> Result<Snmp6Info> {
    let path = Path::new("/proc/net/snmp6");
    let bytes = parse::read_file(path)?;
    let mut info = Snmp6Info::default();

    for (line_number, line) in bytes
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<2>::new(line);
        if fields.len() != 2 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_number + 1,
                msg: "expected counter name and value",
            });
        }
        let value = parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_number + 1,
            msg: "invalid counter value",
        })?;
        set_counter(&mut info, fields[0], value);
    }

    Ok(info)
}

macro_rules! fields {
    ($target:expr, $name:expr, $value:expr, { $($wire:literal => $field:ident),+ $(,)? }) => {
        $(if $name == $wire {
            $target.$field = $value;
            return;
        })+
    };
}

fn set_counter(info: &mut Snmp6Info, name: &[u8], value: u64) {
    if name.starts_with(b"Ip6") {
        let name = &name[3..];
        fields!(info.ip, name, value, {
            b"InReceives" => in_receives, b"InHdrErrors" => in_hdr_errors,
            b"InTooBigErrors" => in_too_big_errors, b"InNoRoutes" => in_no_routes,
            b"InAddrErrors" => in_addr_errors, b"InUnknownProtos" => in_unknown_protos,
            b"InTruncatedPkts" => in_truncated_pkts, b"InDiscards" => in_discards,
            b"InDelivers" => in_delivers, b"OutForwDatagrams" => out_forw_datagrams,
            b"OutRequests" => out_requests, b"OutDiscards" => out_discards,
            b"OutNoRoutes" => out_no_routes, b"ReasmTimeout" => reasm_timeout,
            b"ReasmReqds" => reasm_reqds, b"ReasmOKs" => reasm_oks,
            b"ReasmFails" => reasm_fails, b"FragOKs" => frag_oks,
            b"FragFails" => frag_fails, b"FragCreates" => frag_creates,
            b"InMcastPkts" => in_mcast_pkts, b"OutMcastPkts" => out_mcast_pkts,
            b"InOctets" => in_octets, b"OutOctets" => out_octets,
            b"InMcastOctets" => in_mcast_octets, b"OutMcastOctets" => out_mcast_octets,
            b"InBcastOctets" => in_bcast_octets, b"OutBcastOctets" => out_bcast_octets,
            b"InNoECTPkts" => in_no_ect_pkts, b"InECT1Pkts" => in_ect1_pkts,
            b"InECT0Pkts" => in_ect0_pkts, b"InCEPkts" => in_ce_pkts,
            b"OutTransmits" => out_transmits
        });
    } else if name.starts_with(b"Icmp6") {
        let name = &name[5..];
        fields!(info.icmp, name, value, {
            b"InMsgs" => in_msgs, b"InErrors" => in_errors, b"OutMsgs" => out_msgs,
            b"OutErrors" => out_errors, b"InCsumErrors" => in_csum_errors,
            b"OutRateLimitHost" => out_rate_limit_host, b"InDestUnreachs" => in_dest_unreachs,
            b"InPktTooBigs" => in_pkt_too_bigs, b"InTimeExcds" => in_time_excds,
            b"InParmProblems" => in_parm_problems, b"InEchos" => in_echos,
            b"InEchoReplies" => in_echo_replies, b"InGroupMembQueries" => in_group_memb_queries,
            b"InGroupMembResponses" => in_group_memb_responses,
            b"InGroupMembReductions" => in_group_memb_reductions,
            b"InRouterSolicits" => in_router_solicits,
            b"InRouterAdvertisements" => in_router_advertisements,
            b"InNeighborSolicits" => in_neighbor_solicits,
            b"InNeighborAdvertisements" => in_neighbor_advertisements,
            b"InRedirects" => in_redirects, b"InMLDv2Reports" => in_mldv2_reports,
            b"OutDestUnreachs" => out_dest_unreachs, b"OutPktTooBigs" => out_pkt_too_bigs,
            b"OutTimeExcds" => out_time_excds, b"OutParmProblems" => out_parm_problems,
            b"OutEchos" => out_echos, b"OutEchoReplies" => out_echo_replies,
            b"OutGroupMembQueries" => out_group_memb_queries,
            b"OutGroupMembResponses" => out_group_memb_responses,
            b"OutGroupMembReductions" => out_group_memb_reductions,
            b"OutRouterSolicits" => out_router_solicits,
            b"OutRouterAdvertisements" => out_router_advertisements,
            b"OutNeighborSolicits" => out_neighbor_solicits,
            b"OutNeighborAdvertisements" => out_neighbor_advertisements,
            b"OutRedirects" => out_redirects, b"OutMLDv2Reports" => out_mldv2_reports
        });
    } else if name.starts_with(b"Udp6") {
        let name = &name[4..];
        fields!(info.udp, name, value, {
            b"InDatagrams" => in_datagrams, b"NoPorts" => no_ports, b"InErrors" => in_errors,
            b"OutDatagrams" => out_datagrams, b"RcvbufErrors" => rcvbuf_errors,
            b"SndbufErrors" => sndbuf_errors, b"InCsumErrors" => in_csum_errors,
            b"IgnoredMulti" => ignored_multi, b"MemErrors" => mem_errors
        });
    }
}
