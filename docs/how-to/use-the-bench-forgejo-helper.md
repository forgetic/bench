# Use the bench Forgejo helper

Use `bench forgejo` when a launcher or local workflow needs a disposable Forgejo
instance backed by the shared `bench-forgejo` library.

## Pre-stage binaries

Download the pinned Forgejo server and runner into a deterministic cache before a
workflow starts:

```sh
bench forgejo download --cache-dir .cache/forgejo
```

On success, stdout is stable `key=value` data:

```text
forgejo=/path/to/forgejo-7.0.12-linux-amd64
forgejo_runner=/path/to/forgejo-runner-3.5.1-linux-amd64
```

## Start a server with a runner

Start a fresh server and register a host-mode runner:

```sh
bench forgejo up --with-runner --cache-dir .cache/forgejo
```

`bench forgejo serve` is an alias for `bench forgejo up`. Startup prints one
`key=value` line per value and flushes stdout before blocking:

```text
base_url=http://127.0.0.1:30000
config_path=/tmp/bench-forgejo-.../custom/conf/app.ini
data_dir=/tmp/bench-forgejo-...
runner_name=bench-runner-123-0
```

Launchers should read stdout until they see `base_url=...`, then use that URL for
HTTP calls. For example:

```sh
bench forgejo up --cache-dir .cache/forgejo >forgejo.env &
bench_pid=$!

while ! grep -q '^base_url=' forgejo.env; do
  sleep 0.1
done
base_url=$(sed -n 's/^base_url=//p' forgejo.env | head -n1)

curl "$base_url/api/v1/version"
kill -TERM "$bench_pid"
wait "$bench_pid"
```

Send SIGTERM or SIGINT to the `bench` process when the workflow is done. The CLI
handles the signal, returns from `main`, and lets the library drop paths tear down
the server, runner, and temporary data directory.

`--data-dir` is intentionally not supported in this phase. The library owns a
fresh temporary data directory for every run so fixtures are isolated and cleanup
is automatic.

## Manual integration checks

These checks require network access on first download and therefore are not part
of the default test suite:

1. `bench forgejo download --cache-dir <tmp>` prints both `forgejo=` and
   `forgejo_runner=` paths and populates `<tmp>`.
2. `bench forgejo up --cache-dir <tmp>` prints `base_url=...`; a GET to
   `<base_url>/api/v1/version` returns HTTP 200.
3. `kill -TERM <bench-pid>` makes `bench` exit successfully and releases the
   port through normal drop cleanup.
4. `bench forgejo up --with-runner --cache-dir <tmp>` prints `runner_name=...`
   and tears down cleanly on SIGTERM.
