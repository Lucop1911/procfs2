#[macro_use]
extern crate criterion;

use criterion::Criterion;
use procfs::prelude::*;
use procfs2::proc;

fn bench_proc_uptime(c: &mut Criterion) {
    let mut group = c.benchmark_group("proc::uptime");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(proc::uptime());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(procfs::Uptime::current());
        })
    });
    group.finish();
}

fn bench_proc_loadavg(c: &mut Criterion) {
    let mut group = c.benchmark_group("proc::loadavg");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(proc::loadavg());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(procfs::LoadAverage::current());
        })
    });
    group.finish();
}

fn bench_proc_stat(c: &mut Criterion) {
    let mut group = c.benchmark_group("proc::stat");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(proc::stat());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(procfs::KernelStats::current());
        })
    });
    group.finish();
}

fn bench_proc_cpuinfo(c: &mut Criterion) {
    let mut group = c.benchmark_group("proc::cpuinfo");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(proc::cpuinfo());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(procfs::CpuInfo::current());
        })
    });
    group.finish();
}

fn bench_proc_meminfo(c: &mut Criterion) {
    let mut group = c.benchmark_group("proc::meminfo");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(proc::meminfo());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(procfs::Meminfo::current());
        })
    });
    group.finish();
}

fn bench_proc_process_stat(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    let mut group = c.benchmark_group("Process::stat");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p2.stat());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p0.stat());
        })
    });
    group.finish();
}

fn bench_proc_process_status(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    let mut group = c.benchmark_group("Process::status");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p2.status());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p0.status());
        })
    });
    group.finish();
}

fn bench_proc_maps(c: &mut Criterion) {
    let p2 = proc::Process::current().unwrap();
    let p0 = procfs::process::Process::myself().unwrap();
    let mut group = c.benchmark_group("Process::maps");
    group.bench_function("procfs2", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p2.maps());
        })
    });
    group.bench_function("procfs", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(p0.maps());
        })
    });
    group.finish();
}

fn bench_sys_block_stat(c: &mut Criterion) {
    let devices: Vec<_> = procfs2::sys::BlockDevice::all()
        .filter_map(|r| r.ok())
        .collect();
    if let Some(dev) = devices.first() {
        let mut group = c.benchmark_group("sys::BlockDevice::stat");
        group.bench_function("procfs2", |b| {
            b.iter(|| {
                let _ = std::hint::black_box(dev.stat());
            })
        });
        group.finish();
    }
}

fn bench_sys_net_stats(c: &mut Criterion) {
    let ifaces: Vec<_> = procfs2::sys::NetInterface::all()
        .filter_map(|r| r.ok())
        .collect();
    if let Some(iface) = ifaces.first() {
        let mut group = c.benchmark_group("sys::NetInterface::stats");
        group.bench_function("procfs2", |b| {
            b.iter(|| {
                let _ = std::hint::black_box(iface.stats());
            })
        });
        group.finish();
    }
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
        bench_proc_process_stat,
        bench_proc_process_status,
        bench_proc_maps,
        bench_sys_block_stat,
        bench_sys_net_stats
);
criterion_main!(benches);
