# Miniban

Miniban is an opinionated engineering work model layered over existing issue trackers such as GitHub and Gitea.

This repository is currently a scaffold: the architecture, tooling, and CI are in place, and product behaviour isn't.
See [docs/architecture.md](docs/architecture.md) for how the system is shaped and why.

## Executables

| Executable | Package          | What it does today                                                                     |
| ---------- | ---------------- | -------------------------------------------------------------------------------------- |
| `miniban`  | `miniban-server` | A standalone HTTP server: a placeholder web UI embedded in the binary and a health API |
| `mb`       | `miniban-cli`    | A CLI for developers, scripts, and agents: `--help` and `--version` only               |

`miniban` reads its configuration from the environment:

| Variable       | Purpose             | Default     |
| -------------- | ------------------- | ----------- |
| `MINIBAN_HOST` | Server bind address | `127.0.0.1` |
| `MINIBAN_PORT` | Server port         | `3000`      |
| `RUST_LOG`     | Logging directives  | `info`      |

## Repository structure

```text
crates/          Rust workspace packages (see Crates)
docs/            Architecture document and agent guidance
frontend/        SvelteKit web UI, built with Deno
mise-tasks/      File tasks: the smoke tests
.github/         CI workflow, which runs mise run ci
mise.toml        Tool pins and task definitions
dprint.json      Formatter configuration for everything except Rust
```

## Crates

| Package          | Type    | Purpose                                                               |
| ---------------- | ------- | --------------------------------------------------------------------- |
| `miniban-core`   | Library | Provider-independent core; exports only the product name so far       |
| `miniban-github` | Library | Home of the GitHub issue provider; exports only a display name so far |
| `miniban-gitea`  | Library | Home of the Gitea issue provider; exports only a display name so far  |
| `miniban-server` | Binary  | The `miniban` executable: HTTP routes, embedded frontend, logging     |
| `miniban-cli`    | Binary  | The `mb` executable                                                   |

## Prerequisites

Install [mise](https://mise.jdx.dev), which installs and pins Rust, Deno, dprint, and `gh`. CI uses mise 2026.9.1, so use
that version or newer. You don't need Node.js at any point.

You also need these, which mise doesn't manage:

- `git`
- A linker for Rust, such as the one in Xcode Command Line Tools or `build-essential`
- `bash` and `curl`, for the smoke tests only

## Fresh clone

```sh
git clone https://github.com/miniban-dev/miniban.git
cd miniban
mise trust
mise install
mise run check
mise run test
mise run build
```

`check` and `test` need no frontend build. `build` builds the frontend and then the debug executables, so
`target/debug/miniban` now serves the UI at <http://127.0.0.1:3000>.

Run `mise tasks` to list every task.

## Development

| Task                 | What it runs                                                                  |
| -------------------- | ----------------------------------------------------------------------------- |
| `mise run dev`       | `dev:back` and `dev:front` together; Ctrl-C stops both                        |
| `mise run dev:back`  | The headless API server on port 3000, without the frontend                    |
| `mise run dev:front` | The Vite dev server on <http://localhost:5173>, proxying `/api` to the server |

Open the Vite URL, not port 3000. Frontend changes load through hot module replacement, with no Rust rebuild.

Two things to know about `dev`:

- Rust changes need a manual restart, because there is no file watcher. To restart the server without disturbing Vite,
  run `dev:back` and `dev:front` in separate terminals and restart `dev:back`.
- It leaves `target/debug/miniban` headless until the next `mise run build:back`. A headless server logs a warning at
  startup and answers non-API routes with a 404 saying the frontend isn't included.

If you set `MINIBAN_HOST` or `MINIBAN_PORT`, export them where both processes can see them: the Vite proxy reads them too.

## Release builds

```sh
mise run build:release
```

This builds the frontend, then the release executables, then prints the paths of both, normally
`target/release/miniban` and `target/release/mb`. `miniban` runs from anywhere, with no source tree, Node.js, or Deno.

## Tests and checks

| Task                      | What it does                                                                  |
| ------------------------- | ----------------------------------------------------------------------------- |
| `mise run check`          | `svelte-check`, `deno lint`, and headless Clippy with warnings as errors      |
| `mise run test`           | Workspace tests with the headless server                                      |
| `mise run check:embedded` | Builds the frontend, then runs Clippy and the tests with it embedded          |
| `mise run smoke`          | Tests the release binaries from outside the repository                        |
| `mise run fmt`            | Formats the whole repository                                                  |
| `mise run fmt:check`      | Verifies formatting                                                           |
| `mise run ci`             | `fmt:check`, `check`, `test`, `check:embedded`, `build:release`, then `smoke` |

CI runs `mise run ci`, so you can reproduce a CI failure locally.

The smoke tests don't build anything: `mise run smoke` tests the output of the last `mise run build:release`, so run that
first. They use fixed ports 3101 and 3102, which must be free, and they honour `CARGO_TARGET_DIR`.

## Plain cargo

The tasks are the normal route, but plain `cargo` works too. Activate mise in your shell, or prefix the commands with
`mise exec --`, to get the pinned toolchain.

A server build fails unless `frontend/build` exists or the `headless` feature is on, and a fresh clone has no
`frontend/build`:

```sh
# Clippy and tests
cargo clippy --workspace --all-targets --features miniban-server/headless -- -D warnings
cargo test --workspace --features miniban-server/headless

# The server: either enable headless, or build the frontend first
cargo run -p miniban-server --features headless
mise run build:front && cargo run -p miniban-server

# The CLI needs neither
cargo run -p miniban-cli -- --help
```

Without either one, the build stops with an error telling you to run `mise run build:front`.

**Warning: `--all-features` means no frontend.** `headless` removes functionality, so `--all-features` turns it on and
gives you a server with no UI, and it leaves the embedding code unlinted and untested. Name the features you want
instead.

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
