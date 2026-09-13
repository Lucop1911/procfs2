use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::util::parse;

/// IPv4 packet statistics from the `Ip` section of `/proc/net/snmp`.
#[derive(Debug, Default)]
pub struct IpStats {
    /// Whether IPv4 forwarding is enabled.
    pub forwarding: u64,
    /// Default time-to-live for outgoing IPv4 packets.
    pub default_ttl: u64,
    /// Total IPv4 packets received.
    pub in_receives: u64,
    /// Packets discarded because of IPv4 header errors.
    pub in_hdr_errors: u64,
    /// Packets discarded because of invalid destination addresses.
    pub in_addr_errors: u64,
    /// Packets received for forwarding.
    pub forw_datagrams: u64,
    /// Packets discarded because their protocol was unknown.
    pub in_unknown_protos: u64,
    /// Valid incoming packets discarded for other reasons.
    pub in_discards: u64,
    /// Incoming packets delivered to higher-level protocols.
    pub in_delivers: u64,
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
    /// Fragments created by IPv4 fragmentation.
    pub frag_creates: u64,
    /// Total packets transmitted, including forwarded packets.
    pub out_transmits: u64,
}

/// ICMP packet statistics from the `Icmp` section of `/proc/net/snmp`.
#[derive(Debug, Default)]
pub struct IcmpStats {
    /// ICMP messages received.
    pub in_msgs: u64,
    /// ICMP messages received with errors.
    pub in_errors: u64,
    /// ICMP checksum errors received.
    pub in_csum_errors: u64,
    /// ICMP destination-unreachable messages received.
    pub in_dest_unreachs: u64,
    /// ICMP time-exceeded messages received.
    pub in_time_excds: u64,
    /// ICMP parameter-problem messages received.
    pub in_parm_probs: u64,
    /// ICMP source-quench messages received.
    pub in_src_quenchs: u64,
    /// ICMP redirect messages received.
    pub in_redirects: u64,
    /// ICMP echo requests received.
    pub in_echos: u64,
    /// ICMP echo replies received.
    pub in_echo_reps: u64,
    /// ICMP timestamp requests received.
    pub in_timestamps: u64,
    /// ICMP timestamp replies received.
    pub in_timestamp_reps: u64,
    /// ICMP address-mask requests received.
    pub in_addr_masks: u64,
    /// ICMP address-mask replies received.
    pub in_addr_mask_reps: u64,
    /// ICMP messages sent.
    pub out_msgs: u64,
    /// ICMP messages not sent because of errors.
    pub out_errors: u64,
    /// ICMP messages limited by the global rate limiter.
    pub out_rate_limit_global: u64,
    /// ICMP messages limited by the per-host rate limiter.
    pub out_rate_limit_host: u64,
    /// ICMP destination-unreachable messages sent.
    pub out_dest_unreachs: u64,
    /// ICMP time-exceeded messages sent.
    pub out_time_excds: u64,
    /// ICMP parameter-problem messages sent.
    pub out_parm_probs: u64,
    /// ICMP source-quench messages sent.
    pub out_src_quenchs: u64,
    /// ICMP redirect messages sent.
    pub out_redirects: u64,
    /// ICMP echo requests sent.
    pub out_echos: u64,
    /// ICMP echo replies sent.
    pub out_echo_reps: u64,
    /// ICMP timestamp requests sent.
    pub out_timestamps: u64,
    /// ICMP timestamp replies sent.
    pub out_timestamp_reps: u64,
    /// ICMP address-mask requests sent.
    pub out_addr_masks: u64,
    /// ICMP address-mask replies sent.
    pub out_addr_mask_reps: u64,
}

/// TCP statistics from the `Tcp` section of `/proc/net/snmp`.
#[derive(Debug, Default)]
pub struct TcpStats {
    /// TCP retransmission timeout algorithm.
    pub rto_algorithm: u64,
    /// Minimum TCP retransmission timeout in milliseconds.
    pub rto_min: u64,
    /// Maximum TCP retransmission timeout in milliseconds.
    pub rto_max: u64,
    /// Maximum supported TCP connections, or `-1` when dynamic.
    pub max_conn: i64,
    /// Connections opened actively.
    pub active_opens: u64,
    /// Connections opened passively.
    pub passive_opens: u64,
    /// Failed TCP connection attempts.
    pub attempt_fails: u64,
    /// Established connections reset.
    pub estab_resets: u64,
    /// Currently established or closing connections.
    pub curr_estab: u64,
    /// TCP segments received.
    pub in_segs: u64,
    /// TCP segments sent.
    pub out_segs: u64,
    /// TCP segments retransmitted.
    pub retrans_segs: u64,
    /// TCP segments received with errors.
    pub in_errs: u64,
    /// TCP segments sent with the RST flag.
    pub out_rsts: u64,
    /// TCP checksum errors received.
    pub in_csum_errors: u64,
}

/// UDP statistics from the `Udp` section of `/proc/net/snmp`.
#[derive(Debug, Default)]
pub struct UdpStats {
    /// UDP datagrams delivered to applications.
    pub in_datagrams: u64,
    /// UDP datagrams received for an unused port.
    pub no_ports: u64,
    /// UDP receive errors other than unused ports.
    pub in_errors: u64,
    /// UDP datagrams sent.
    pub out_datagrams: u64,
    /// Receive buffer errors.
    pub rcvbuf_errors: u64,
    /// Send buffer errors.
    pub sndbuf_errors: u64,
    /// UDP checksum errors received.
    pub in_csum_errors: u64,
    /// Multicast datagrams ignored.
    pub ignored_multi: u64,
    /// UDP memory allocation errors.
    pub mem_errors: u64,
}

/// IPv4 protocol statistics read from `/proc/net/snmp`.
#[derive(Debug, Default)]
pub struct SnmpInfo {
    pub ip: IpStats,
    pub icmp: IcmpStats,
    pub tcp: TcpStats,
    pub udp: UdpStats,
}

/// Alias for [`SnmpInfo`].
pub type Snmp = SnmpInfo;

/// Reads IPv4 protocol statistics from `/proc/net/snmp`.
///
/// Counters are matched by their header names so newer kernels can add
/// counters without changing the parser.
pub fn snmp() -> Result<SnmpInfo> {
    let path = Path::new("/proc/net/snmp");
    let bytes = parse::read_file(path)?;
    let mut lines = bytes
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty());
    let mut info = SnmpInfo::default();

    while let Some(header) = lines.next() {
        let values = lines
            .next()
            .ok_or_else(|| parse_error(path, "missing value line"))?;
        let (protocol, header_fields) = parse::split_at_byte(header, b':');
        let (value_protocol, value_fields) = parse::split_at_byte(values, b':');
        if parse::trim(protocol) != parse::trim(value_protocol) {
            return Err(parse_error(path, "section names do not match"));
        }

        let headers = parse::SplitFields::<32>::new(header_fields);
        let values = parse::SplitFields::<32>::new(value_fields);
        if headers.len() != values.len() {
            return Err(parse_error(path, "header and value counts differ"));
        }

        match parse::trim(protocol) {
            b"Ip" => parse_ip(&mut info.ip, &headers, &values)?,
            b"Icmp" => parse_icmp(&mut info.icmp, &headers, &values)?,
            b"Tcp" => parse_tcp(&mut info.tcp, &headers, &values)?,
            b"Udp" => parse_udp(&mut info.udp, &headers, &values)?,
            _ => {}
        }
    }

    Ok(info)
}

fn parse_error(path: &Path, msg: &'static str) -> Error {
    Error::Parse {
        path: PathBuf::from(path),
        line: 0,
        msg,
    }
}

fn find_field(headers: &parse::SplitFields<'_, 32>, name: &[u8]) -> Option<usize> {
    headers.iter().position(|header| *header == name)
}

fn value(
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
    name: &[u8],
) -> Result<Option<u64>> {
    let Some(index) = find_field(headers, name) else {
        return Ok(None);
    };
    parse::parse_dec_u64(values[index])
        .map(Some)
        .map_err(|_| parse_error(Path::new("/proc/net/snmp"), "invalid counter"))
}

fn signed_value(
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
    name: &[u8],
) -> Result<Option<i64>> {
    let Some(index) = find_field(headers, name) else {
        return Ok(None);
    };
    std::str::from_utf8(values[index])
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Some)
        .ok_or_else(|| parse_error(Path::new("/proc/net/snmp"), "invalid signed counter"))
}

macro_rules! parse_fields {
    ($target:expr, $headers:expr, $values:expr, { $($field:ident => $name:literal),+ $(,)? }) => {
        $(
            if let Some(value) = value($headers, $values, $name.as_bytes())? {
                $target.$field = value;
            }
        )+
    };
}

fn parse_ip(
    target: &mut IpStats,
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
) -> Result<()> {
    parse_fields!(target, headers, values, {
        forwarding => "Forwarding", default_ttl => "DefaultTTL", in_receives => "InReceives",
        in_hdr_errors => "InHdrErrors", in_addr_errors => "InAddrErrors",
        forw_datagrams => "ForwDatagrams", in_unknown_protos => "InUnknownProtos",
        in_discards => "InDiscards", in_delivers => "InDelivers", out_requests => "OutRequests",
        out_discards => "OutDiscards", out_no_routes => "OutNoRoutes", reasm_timeout => "ReasmTimeout",
        reasm_reqds => "ReasmReqds", reasm_oks => "ReasmOKs", reasm_fails => "ReasmFails",
        frag_oks => "FragOKs", frag_fails => "FragFails", frag_creates => "FragCreates",
        out_transmits => "OutTransmits"
    });
    Ok(())
}

fn parse_icmp(
    target: &mut IcmpStats,
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
) -> Result<()> {
    parse_fields!(target, headers, values, {
        in_msgs => "InMsgs", in_errors => "InErrors", in_csum_errors => "InCsumErrors",
        in_dest_unreachs => "InDestUnreachs", in_time_excds => "InTimeExcds",
        in_parm_probs => "InParmProbs", in_src_quenchs => "InSrcQuenchs",
        in_redirects => "InRedirects", in_echos => "InEchos", in_echo_reps => "InEchoReps",
        in_timestamps => "InTimestamps", in_timestamp_reps => "InTimestampReps",
        in_addr_masks => "InAddrMasks", in_addr_mask_reps => "InAddrMaskReps",
        out_msgs => "OutMsgs", out_errors => "OutErrors",
        out_rate_limit_global => "OutRateLimitGlobal", out_rate_limit_host => "OutRateLimitHost",
        out_dest_unreachs => "OutDestUnreachs", out_time_excds => "OutTimeExcds",
        out_parm_probs => "OutParmProbs", out_src_quenchs => "OutSrcQuenchs",
        out_redirects => "OutRedirects", out_echos => "OutEchos", out_echo_reps => "OutEchoReps",
        out_timestamps => "OutTimestamps", out_timestamp_reps => "OutTimestampReps",
        out_addr_masks => "OutAddrMasks", out_addr_mask_reps => "OutAddrMaskReps"
    });
    Ok(())
}

fn parse_tcp(
    target: &mut TcpStats,
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
) -> Result<()> {
    parse_fields!(target, headers, values, {
        rto_algorithm => "RtoAlgorithm", rto_min => "RtoMin", rto_max => "RtoMax",
        active_opens => "ActiveOpens", passive_opens => "PassiveOpens",
        attempt_fails => "AttemptFails", estab_resets => "EstabResets", curr_estab => "CurrEstab",
        in_segs => "InSegs", out_segs => "OutSegs", retrans_segs => "RetransSegs",
        in_errs => "InErrs", out_rsts => "OutRsts", in_csum_errors => "InCsumErrors"
    });
    if let Some(value) = signed_value(headers, values, b"MaxConn")? {
        target.max_conn = value;
    }
    Ok(())
}

fn parse_udp(
    target: &mut UdpStats,
    headers: &parse::SplitFields<'_, 32>,
    values: &parse::SplitFields<'_, 32>,
) -> Result<()> {
    parse_fields!(target, headers, values, {
        in_datagrams => "InDatagrams", no_ports => "NoPorts", in_errors => "InErrors",
        out_datagrams => "OutDatagrams", rcvbuf_errors => "RcvbufErrors",
        sndbuf_errors => "SndbufErrors", in_csum_errors => "InCsumErrors",
        ignored_multi => "IgnoredMulti", mem_errors => "MemErrors"
    });
    Ok(())
}
