## System-wide

- [ x ] `/proc/cmdline` — kernel command line. Space-separated boot parameters. Trivial split on spaces.
- [ x ] `/proc/devices` — character and block device major numbers. Two sections, `Character devices:` / `Block devices:`, lines are `<major> <name>`.
- [ x ] `/proc/filesystems` — registered filesystems. Lines are `<flag>\t<fsname>` where flag is `nodev` (no block device) or empty.
- [ ] `/proc/swaps` — swap devices. Header + lines: `Filename  Type  Size  Used  Priority`.
- [ ] `/proc/partitions` — partition table. Header + lines: `major  minor  #blocks  name`.
- [ ] `/proc/diskstats` — per-disk I/O statistics. Fixed 18-field layout (documented in kernel docs `iostats.txt`).
- [ ] `/proc/vmstat` — virtual memory statistics. `key value` lines.
- [ ] `/proc/zoneinfo` — per-zone memory info. Multi-line zones (`Node 0, zone   DMA`), per-page-type and per-free-list sections.
- [ ] `/proc/buddyinfo` — buddy allocator free lists. Per-zone lines of free-page counts per order.
- [ ] `/proc/pagetypeinfo` — page block allocator types. Per-zone block-type/order matrix.
- [ ] `/proc/softirqs` — per-CPU softirq counters. `softirq_name: cpu0 cpu1 ...` lines.
- [ ] `/proc/interrupts` — per-CPU IRQ counters. `irq: cpu0 cpu1 ... [device]` lines.
- [ ] `/proc/iomem` — physical memory map. `start-end : description`, one tree of indented regions.
- [ ] `/proc/ioports` — I/O port regions, same indented-tree format.
- [ ] `/proc/misc` — misc (miscellaneous) character devices. `major name` lines.
- [ ] `/proc/modules` — loaded modules. `name size refcount deps state address` per line.
- [ ] `/proc/slabinfo` — slab cache info. Version header, then `name active_objs num_objs objsize ...`.
- [ ] `/proc/locks` — open file locks. Fixed-ish text columns (`1: POSIX  ADVISORY  WRITE ...`).
- [ ] `/proc/crypto` — registered crypto algorithms. Multi-line records split by blank lines.
- [ ] `/proc/consoles` — registered console devices. `name flags operation `tty`/`/dev`.
- [ ] `/proc/kallsyms` — kernel symbol table. `address type name` lines (needs root for addresses, `CONFIG_KALLSYMS`).
- [ ] `/proc/schedstat` — per-CPU + per-task scheduler statistics. Three-line groups.
- [ ] `/proc/timer_list` — kernel timer list. Multi-line debug output.
- [ ] `/proc/keys` and `/proc/key-users` — kernel keyring info. Structured tables.
- [ ] `/proc/sysvipc/{msg,sem,shm}` — SysV IPC objects. `key msqid perms cbytes ...` column tables.
- [ ] `/proc/pressure/cpu`, `/proc/pressure/memory`, `/proc/pressure/io` — PSI. `some avg10=.. avg60=.. avg300=.. total=..` (plus `full` line for mem/io).

### Symlinks / special

- [ ] `/proc/thread-self` — symlink to `/proc/self/task/<tid>`. Cheap, but worth a `thread_self()` accessor.
- [ ] `/proc/self`, `/proc/<pid>/root`, `/proc/<pid>/mounts` (symlink to `mountinfo`-style output — already parseable via `mountinfo`).

## /proc/net

- [ ] `/proc/net/ipv6_route` — IPv6 routing table. Hex fields: `dest src ... dev ... flags ...`. Fixed 10-field layout.
- [ ] `/proc/net/snmp` — TCP/IP protocol counters. Alternating `key:` header line + values line.
- [ ] `/proc/net/snmp6` — IPv6 counters. Single `key<num> value` per line.
- [ ] `/proc/net/netlink` — netlink socket table. `sk  Eth  Pid  Groups  Rmem  Wmem  Dump  Locks  Drops`.
- [ ] `/proc/net/protocol` — registered network protocols. Name/type/memory columns.
- [ ] `/proc/net/wireless` — wireless interface stats. Per-interface line after a header.
- [ ] `/proc/net/ppp` — PPP statistics. Per-channel block text.
- [ ] `/proc/net/raw` and `/proc/net/raw6` — raw sockets, same layout as tcp/udp.
- [ ] `/proc/net/icmp` and `/proc/net/icmp6` — ICMP sockets, same layout as tcp/udp.
- [ ] `/proc/net/fib_trie` and `/proc/net/fib_triestat` — routing trie dump (debug-oriented).

## Per-process (`/proc/<pid>/`)

- [ ] `statm` — memory size in pages: `size resident shared text lib data dt`.
- [ ] `oom_score`, `oom_score_adj`, `oom_adj` — single integers.
- [ ] `comm` — process name (truncated, no newline handling caveats).
- [ ] `cpuset` — cpuset path.
- [ ] `sched` — scheduler attributes. `name (key): value` lines.
- [ ] `schedstat` — three numbers: `running time, waiting time, switch count`.
- [ ] `syscall` — `syscall_nr arg0 arg1 arg2 ...` or `running`/`blocked` in some states.
- [ ] `wchan` — kernel symbol the process is blocked on.
- [ ] `stack` — kernel stack trace (privileged).
- [ ] `auxv` — ELF auxiliary vector. Binary `[u8; 16]` records (`value + ptr` pairs); endianness per-arch.
- [ ] `numa_maps` — per-mapping NUMA policy/counters.
- [ ] `pagemap` — binary bitfield of PFN/swap entry per page (privileged, 8 bytes per page).
- [ ] `uid_map`, `gid_map`, `projid_map` — `inside outside length` lines.
- [ ] `setgroups` — `allow`/`deny` single word.
- [ ] `personality` — single int.
- [ ] `loginuid`, `sessionid` — single ints.
- [ ] `timers` — POSIX timers (per-signal counts).
- [ ] `timerslack_ns` — single int.
- [ ] `autogroup` — `nice <value>` line.
- [ ] `coredump_filter` — bitmask int.
- [ ] `latency` — latency statistics (legacy).
- [ ] `fdinfo/<fd>` — per-open-file info. `pos: flags: mnt_id: ino:` key-value lines; `eventpoll:`/`tty:` sections for special fds.

### Per-thread (`/proc/<pid>/task/<tid>/`)

Mirror of the per-process files above. Only needed if thread-level accessors are wanted
(beyond the existing `threads()` enumeration).

## Explicitly out of scope (consider documenting)

- `/proc/sys/**` — thousands of sysctl knobs; a separate sysctl API.
- `/proc/kcore` — live kernel memory image, binary + privileged.
- `/proc/kmsg` — kernel log (use `dmesg`/`syslog()`).
- `/proc/acpi/**`, `/proc/scsi/**` — legacy, superseded by `/sys` and `lspci`/`lsscsi`.

---

## Already implemented

- `cgroups`, `cpuinfo`, `devices`, `filesystems`, `loadavg`, `meminfo`, `mounts`, `stat`, `uptime`, `version`
- `net/{arp,dev,route,tcp,tcp6,udp,udp6,unix}`
- per-process: `stat`, `status`, `cmdline`, `environ`, `exe`, `cwd`, `maps`, `smaps`,
  `smaps_rollup`, `fd`, `io`, `limits`, `mountinfo`, `cgroup`, `ns`, `task` (thread enumeration)
