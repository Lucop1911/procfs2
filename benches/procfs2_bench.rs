#[macro_use]
extern crate criterion;

use criterion::Criterion;
use procfs::prelude::*;
use procfs2::proc;

macro_rules! bench_pair {
    ($c:expr, $name:literal, $procfs2:expr, $procfs:expr) => {{
        let mut group = $c.benchmark_group($name);
        group.bench_function("procfs2", |b| {
            b.iter(|| {
                let _ = std::hint::black_box($procfs2);
            })
        });
        group.bench_function("procfs", |b| {
            b.iter(|| {
                let _ = std::hint::black_box($procfs);
            })
        });
        group.finish();
    }};
}

fn bench_proc_uptime(c: &mut Criterion) {
    bench_pair!(c, "proc::uptime", proc::uptime(), procfs::Uptime::current());
}

fn bench_proc_loadavg(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::loadavg",
        proc::loadavg(),
        procfs::LoadAverage::current()
    );
}

fn bench_proc_stat(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::stat",
        proc::stat(),
        procfs::KernelStats::current()
    );
}

fn bench_proc_cpuinfo(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::cpuinfo",
        proc::cpuinfo(),
        procfs::CpuInfo::current()
    );
}

fn bench_proc_meminfo(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::meminfo",
        proc::meminfo(),
        procfs::Meminfo::current()
    );
}

fn bench_proc_cgroups(c: &mut Criterion) {
    bench_pair!(c, "proc::cgroups", proc::cgroups(), procfs::cgroups());
}

fn bench_proc_cmdline(c: &mut Criterion) {
    bench_pair!(c, "proc::cmdline", proc::cmdline(), procfs::cmdline());
}

fn bench_proc_crypto(c: &mut Criterion) {
    bench_pair!(c, "proc::crypto", proc::crypto(), procfs::crypto());
}

fn bench_proc_devices(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::devices",
        proc::devices(),
        procfs::Devices::current()
    );
}

fn bench_proc_diskstats(c: &mut Criterion) {
    bench_pair!(c, "proc::diskstats", proc::diskstats(), procfs::diskstats());
}

fn bench_proc_iomem(c: &mut Criterion) {
    bench_pair!(c, "proc::iomem", proc::iomem(), procfs::iomem());
}

fn bench_proc_locks(c: &mut Criterion) {
    bench_pair!(c, "proc::locks", proc::locks(), procfs::locks());
}

fn bench_proc_modules(c: &mut Criterion) {
    bench_pair!(c, "proc::modules", proc::modules(), procfs::modules());
}

fn bench_proc_mounts(c: &mut Criterion) {
    bench_pair!(c, "proc::mounts", proc::mounts(), procfs::mounts());
}

fn bench_proc_partitions(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::partitions",
        proc::partitions(),
        procfs::partitions()
    );
}

fn bench_proc_vmstat(c: &mut Criterion) {
    bench_pair!(c, "proc::vmstat", proc::vmstat(), procfs::vmstat());
}

fn bench_proc_net_dev(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::net::dev",
        proc::net::dev(),
        procfs::net::dev_status()
    );
}

fn bench_proc_net_route(c: &mut Criterion) {
    bench_pair!(
        c,
        "proc::net::route",
        proc::net::route(),
        procfs::net::route()
    );
}

fn bench_proc_net_arp(c: &mut Criterion) {
    bench_pair!(c, "proc::net::arp", proc::net::arp(), procfs::net::arp());
}

fn bench_proc_net_tcp(c: &mut Criterion) {
    bench_pair!(c, "proc::net::tcp", proc::net::tcp(), procfs::net::tcp());
}

fn bench_proc_net_tcp6(c: &mut Criterion) {
    bench_pair!(c, "proc::net::tcp6", proc::net::tcp6(), procfs::net::tcp6());
}

fn bench_proc_net_udp(c: &mut Criterion) {
    bench_pair!(c, "proc::net::udp", proc::net::udp(), procfs::net::udp());
}

fn bench_proc_net_udp6(c: &mut Criterion) {
    bench_pair!(c, "proc::net::udp6", proc::net::udp6(), procfs::net::udp6());
}

fn bench_proc_net_unix(c: &mut Criterion) {
    bench_pair!(c, "proc::net::unix", proc::net::unix(), procfs::net::unix());
}

fn bench_proc_process_stat(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::stat", p2.stat(), p0.stat());
}

fn bench_proc_process_status(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::status", p2.status(), p0.status());
}

fn bench_proc_process_cmdline(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::cmdline", p2.cmdline(), p0.cmdline());
}

fn bench_proc_process_environ(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::environ", p2.environ(), p0.environ());
}

fn bench_proc_process_exe(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::exe", p2.exe(), p0.exe());
}

fn bench_proc_process_cwd(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::cwd", p2.cwd(), p0.cwd());
}

fn bench_proc_process_io(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::io", p2.io(), p0.io());
}

fn bench_proc_process_limits(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::limits", p2.limits(), p0.limits());
}

fn bench_proc_process_maps(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::maps", p2.maps(), p0.maps());
}

fn bench_proc_process_smaps(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::smaps", p2.smaps(), p0.smaps());
}

fn bench_proc_process_smaps_rollup(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(
        c,
        "Process::smaps_rollup",
        p2.smaps_rollup(),
        p0.smaps_rollup()
    );
}

fn bench_proc_process_mountinfo(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::mountinfo", p2.mountinfo(), p0.mountinfo());
}

fn bench_proc_process_namespaces(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::namespaces", p2.namespaces(), p0.namespaces());
}

fn bench_proc_process_cgroup(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    bench_pair!(c, "Process::cgroup", p2.cgroup(), p0.cgroups());
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(std::time::Duration::from_secs(1));
    targets =
        bench_proc_uptime,
        bench_proc_loadavg,
        bench_proc_stat,
        bench_proc_cpuinfo,
        bench_proc_meminfo,
        bench_proc_cgroups,
        bench_proc_cmdline,
        bench_proc_crypto,
        bench_proc_devices,
        bench_proc_diskstats,
        bench_proc_iomem,
        bench_proc_locks,
        bench_proc_modules,
        bench_proc_mounts,
        bench_proc_partitions,
        bench_proc_vmstat,
        bench_proc_net_dev,
        bench_proc_net_route,
        bench_proc_net_arp,
        bench_proc_net_tcp,
        bench_proc_net_tcp6,
        bench_proc_net_udp,
        bench_proc_net_udp6,
        bench_proc_net_unix,
        bench_proc_process_stat,
        bench_proc_process_status,
        bench_proc_process_cmdline,
        bench_proc_process_environ,
        bench_proc_process_exe,
        bench_proc_process_cwd,
        bench_proc_process_io,
        bench_proc_process_limits,
        bench_proc_process_maps,
        bench_proc_process_smaps,
        bench_proc_process_smaps_rollup,
        bench_proc_process_mountinfo,
        bench_proc_process_namespaces,
        bench_proc_process_cgroup
);
criterion_main!(benches);
