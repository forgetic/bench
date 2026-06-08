# Forgejo fixture reference

The `bench forgejo` CLI is a thin wrapper over the `bench-forgejo` workspace
library. It manages throwaway Forgejo processes for tests and local automation.

## Pinned binaries

`bench-forgejo` downloads checksum-verified Linux amd64 release assets when no
usable override or cached binary exists:

- Forgejo server `7.0.12`, SHA-256
  `ecd25535250aeb8073fdef1a0c9e92f288de1c0cdde24c95a3b61ead6bc9cf7c`.
- Forgejo runner `3.5.1`, SHA-256
  `e2f36aa8149a0e883b5713398aa185c88a827fc0527d5cd2e2b05b88c9ba0b36`.

`bench forgejo download` resolves both binaries and prints their paths as
`forgejo=...` and `forgejo_runner=...`.

## Environment variables

Canonical variables are preferred:

- `BENCH_FORGEJO_CACHE_DIR`: shared binary cache directory.
- `BENCH_FORGEJO_GOMAXPROCS`: value assigned to `GOMAXPROCS` for spawned Forgejo
  server commands. An empty value opts out of setting `GOMAXPROCS`; otherwise the
  library defaults to `2`.

Deprecated aliases are still accepted by the library after canonical names:

- `TEMPER_FORGEJO_CACHE_DIR`
- `TEMPER_FORGEJO_GOMAXPROCS`

The download layer also supports canonical server and runner override families
such as `BENCH_FORGEJO_BINARY`, `BENCH_FORGEJO_VERSION`, `BENCH_FORGEJO_URL`,
`BENCH_FORGEJO_SHA256`, and the corresponding `BENCH_FORGEJO_RUNNER_*` variables;
their `TEMPER_FORGEJO*` aliases remain supported.

## Cache resolution order

The binary cache directory is selected in this order:

1. `BENCH_FORGEJO_CACHE_DIR`.
2. `TEMPER_FORGEJO_CACHE_DIR`.
3. `.cache/forgejo` under the nearest ancestor `Cargo.toml` containing a
   `[workspace]` table.
4. `$XDG_CACHE_HOME/bench-forgejo`.
5. `$HOME/.cache/bench-forgejo`.

This cache holds downloaded Forgejo and forgejo-runner binaries only. Each
`ForgejoServer::start()` run creates a fresh temporary data directory for runtime
state, repositories, logs, and SQLite data; the CLI intentionally does not expose
`--data-dir`.

## Teardown model

`ForgejoServer` and `ForgejoRunner` are kill-on-drop fixtures. `bench forgejo up`
keeps those handles alive while it waits for SIGINT or SIGTERM. On signal, the
process returns from `main`, drops the runner and server, kills their child
processes, and removes temporary state on a best-effort basis.
