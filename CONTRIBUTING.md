# Contributing to procfs2

Thanks for your interest. This document covers how to report problems, what to check before opening a PR, and the conventions the codebase follows.

## Reporting issues

- **Bug reports** — include the kernel version (`uname -r`), your Rust version, and, if a parse fails, the raw `/proc` or `/sys` contents that triggered it. Live tests in `tests/` only verify on the machine they run on, so formats can differ on your kernel.
- **Feature requests** — point at the `/proc` or `/sys` file you want parsed. If it's a new module, mention what the output is consumed by; that helps prioritization.

## Development setup

Linux is required — the crate only supports Linux and refuses to compile elsewhere. No special setup beyond a recent Rust toolchain:

```bash
git clone https://github.com/lucop1911/procfs2.git
cd procfs2
cargo build --features "async,serde,macros,watch"
```

The MSRV is Rust 1.86, so code must compile on both 1.86 and the current stable release.

## Code conventions

- **Performance first.** Prefer zero-copy parsing over allocation. Read `src/util/parse.rs` before writing a parser — the crate has helpers such as `SplitFields<N>`, `split_spaces`, and fast hex/decimal parsers that most new code should use.
- **Parse from bytes, not strings.** The `ParseFromBytes` trait routes through byte slices to avoid UTF-8 conversion. New parsers should operate on `&[u8]`.
- **Doc comments should read naturally.** Match the tone of existing docs in `src/proc/` — factual, terse, no marketing language. Describe what the field *is* and where it comes from.
- **Structs describe what a type IS, not where it comes from.** The module path already provides context (`proc::IoMem` is self-documenting). Don't prefix names with the module (`MiscDevice`, `SystemStat`) and don't append `Entry` to flat list items (`IoMem` not `IoMemEntry`).
- **Keep the public API lean.** Only expose what users need. Internal helpers stay private.

## Adding a new `/proc` or `/sys` module

1. Create the parser under `src/proc/<name>.rs` or `src/sys/<name>.rs`, and register it in the module root.
2. Add three live tests to `tests/integration.rs` exercising the parser against the real filesystem (see the existing entries for the pattern — each module gets three).
3. Match how errors are handled: missing optional fields default to zero or `None` rather than failing, since `/proc` format evolution favors tolerance over strictness.
4. If your parser targets a procfs-core equivalent, run the criterion benchmarks to confirm you aren't regressing on allocation.

## Submitting a pull request

Outside collaborators don't have write access, so work happens on a fork.

1. Fork the repo on GitHub.
2. Clone your fork and add a branch for your change:
   ```bash
   git clone https://github.com/<your-username>/procfs2.git
   cd procfs2
   git checkout -b add-vec-quick
   ```
3. Commit your work with a descriptive message (see below).
4. Push the branch to your fork:
   ```bash
   git push -u origin add-vec-quick
   ```
5. Open a PR from that branch to `main` on the upstream repo. If your change addresses an issue, reference it in the PR body (e.g. `Closes #12`).

## Commit messages

Messages follow the `type: summary` format used throughout the existing history — a lowercase type, a colon, then a terse summary of the change:

```text
add: support for /proc/<pid>/oom_score and oom_score_adj
fix: work around inotify not firing on some /proc files
tests: add live tests for pagetypeinfo blocks
refactor: replace manual hex parsing with parse_hex_fast
docs: expand the meminfo field docs
```

This is the loose convention, not the full Conventional Commits spec — types and scope are left to your judgment. There's no CI commit-linting, so don't stress over it.

## Verification

Run all three before opening a PR:

```bash
cargo t     # test --all-features --all-targets --quiet
cargo c     # clippy --all-features --all-targets
cargo fmt --all
```

These wrap the aliases defined in `.cargo/config.toml`. CI runs the same checks on 1.86 and stable, so expect the pipeline to fail if any of them are red locally.

## Project layout

- `src/proc/` — system-wide and per-process `/proc` parsers
- `src/sys/` — `/sys` filesystem entries (block devices, CPUs, network, power)
- `src/util/` — parsing toolkit and unit newtypes
- `src/error.rs` — error types and the `KernelVersion` helper
- `src/async_helpers.rs` — feature-gated async I/O (`async` feature)
- `src/watch/` — feature-gated inotify watcher and sampler (`watch` feature)
- `procfs2-macros/` — the `ProcKeyValue` derive macro (`macros` feature)
- `tests/` — live integration tests against `/proc` and `/sys`
- `benches/` — criterion comparisons against the `procfs` crate
- `examples/` — demo CLI and feature examples

## License

By contributing you agree that your work will be licensed under the same [MIT license](README.md#license) as the crate.