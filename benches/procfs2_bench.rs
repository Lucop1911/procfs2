#[macro_use]
extern crate criterion;

use criterion::{black_box, Criterion};
use procfs2::proc;

fn bench_proc_uptime(c: &mut Criterion) {
    c.bench_function("proc::uptime", |b| {
        b.iter(|| {
            let _ = black_box(proc::uptime());
        })
    });
}

fn bench_proc_loadavg(c: &mut Criterion) {
    c.bench_function("proc::loadavg", |b| {
        b.iter(|| {
            let _ = black_box(proc::loadavg());
        })
    });
}

fn bench_proc_stat(c: &mut Criterion) {
    c.bench_function("proc::stat", |b| {
        b.iter(|| {
            let _ = black_box(proc::stat());
        })
    });
}

fn bench_proc_cpuinfo(c: &mut Criterion) {
    c.bench_function("proc::cpuinfo", |b| {
        b.iter(|| {
            let _ = black_box(proc::cpuinfo());
        })
    });
}

fn bench_proc_meminfo(c: &mut Criterion) {
    c.bench_function("proc::meminfo", |b| {
        b.iter(|| {
            let _ = black_box(proc::meminfo());
        })
    });
}

fn bench_proc_process_stat(c: &mut Criterion) {
    let p = proc::Process::current().unwrap();
    c.bench_function("Process::stat", |b| {
        b.iter(|| {
            let _ = black_box(p.stat());
        })
    });
}

fn bench_proc_process_status(c: &mut Criterion) {
    let p = proc::Process::current().unwrap();
    c.bench_function("Process::status", |b| {
        b.iter(|| {
            let _ = black_box(p.status());
        })
    });
}

fn bench_proc_maps(c: &mut Criterion) {
    let p = proc::Process::current().unwrap();
    c.bench_function("Process::maps", |b| {
        b.iter(|| {
            let _ = black_box(p.maps());
        })
    });
}

fn bench_sys_block_stat(c: &mut Criterion) {
    let devices: Vec<_> = procfs2::sys::BlockDevice::all().filter_map(|r| r.ok()).collect();
    if let Some(dev) = devices.first() {
        c.bench_function("sys::BlockDevice::stat", |b| {
            b.iter(|| {
                let _ = black_box(dev.stat());
            })
        });
    }
}

fn bench_sys_net_stats(c: &mut Criterion) {
    let ifaces: Vec<_> = procfs2::sys::NetInterface::all().filter_map(|r| r.ok()).collect();
    if let Some(iface) = ifaces.first() {
        c.bench_function("sys::NetInterface::stats", |b| {
            b.iter(|| {
                let _ = black_box(iface.stats());
            })
        });
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
