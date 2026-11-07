# Architecture

This document describes the scaffold as built. Product behaviour, such as the domain model and real provider
integrations, doesn't exist yet. See the [README](../README.md) for commands.

## Components

```text
browser                    miniban (crates/miniban-server)
  SvelteKit SPA  -- HTTP -->  /api/*         router in app.rs
  (frontend/)                 everything else  assets embedded by frontend.rs

mb (crates/miniban-cli)    no network access: --help and --version only
```

- **Frontend (`frontend/`):** a client-rendered SvelteKit SPA built with Deno by the static adapter. It's one placeholder
  page showing the product name, a description, and the server's status.
- **Server (`miniban-server`):** Axum on Tokio. `app.rs` defines the HTTP contract, `frontend.rs` serves the embedded
  files, `config.rs` reads the environment, and `main.rs` sets up logging and graceful shutdown on SIGINT and SIGTERM.
- **CLI (`miniban-cli`):** a `clap` parser with no subcommands.
- **Core (`miniban-core`):** the crate the others share. It exports only `PRODUCT_NAME` so far.
- **Provider crates (`miniban-github`, `miniban-gitea`):** placeholders. See [Issue provider abstraction](#issue-provider-abstraction).

## Dependency boundaries

| Package          | May depend on            | Depends on today |
| ---------------- | ------------------------ | ---------------- |
| `miniban-core`   | No Miniban crates        | Nothing          |
| `miniban-github` | `miniban-core`           | `miniban-core`   |
| `miniban-gitea`  | `miniban-core`           | `miniban-core`   |
| `miniban-server` | Core and provider crates | `miniban-core`   |
| `miniban-cli`    | Core and provider crates | `miniban-core`   |

Dependencies flow one way, from executables to libraries, so that the domain stays independent of frameworks and
providers:

- Core doesn't depend on Axum, Tokio, Svelte, or provider SDKs.
- Provider crates don't depend on each other, which keeps each provider's details isolated.
- No library depends on an executable package.

Nothing enforces these rules automatically. Review changes to the `Cargo.toml` files, and inspect the graph with
`cargo tree --workspace --depth 1 --features miniban-server/headless`.

All packages share one `Cargo.lock`, one version, and the dependency versions in the workspace manifest.

## Frontend-to-server communication

The contract is JSON over HTTP on relative paths, so the frontend is always same-origin and needs no CORS setup.

| Request                    | Response                                              |
| -------------------------- | ----------------------------------------------------- |
| `GET /api/health`          | 200 with `{"status":"ok"}`                            |
| Other method on that route | 405 with `{"error":"method not allowed"}`             |
| Any other `/api` route     | 404 with `{"error":"not found"}`, never the frontend  |
| Path with an empty segment | 404 with `{"error":"not found"}`, never the frontend  |
| Path to an embedded file   | The file                                              |
| Any other non-API path     | 200 with `index.html`, so browser-side routes refresh |
| Any non-API path, headless | 404 in plain text saying the frontend isn't included  |

The only call the frontend makes today is `GET /api/health`, which fills in its status indicator.

## Issue provider abstraction

**There is no provider abstraction yet.** No trait, shared interface, or domain model exists, and that is deliberate: the
scaffold leaves them to be designed with the product rather than guessing at them now.

What exists is the place where one will go. `miniban-github` and `miniban-gitea` are compiling libraries that each export
only `provider::display_name()`. They talk to no external service, the two share no interface, and neither executable
depends on them yet, so no provider code is reachable from `miniban` or `mb`.

The intended shape is the boundary above: the provider-independent model lives in `miniban-core`, and each provider's
details stay in its own crate.

## Asset embedding

`frontend.rs` uses `rust-embed` to compile `frontend/build` into the `miniban` executable. The `debug-embed` feature makes
debug builds embed too, so debug and release share one serving path and neither reads frontend files from disk at runtime.

Cargo never invokes the frontend toolchain: building the frontend is an explicit `mise run build:front`. Two things follow:

- `crates/miniban-server/build.rs` fails the build with an error naming that task if `frontend/build/index.html` is
  missing, so a build can't silently embed an empty application.
- A Rust build embeds whichever frontend was built last, so a stale `frontend/build` produces a stale UI.

Files under `_app/immutable/` have content-hashed names and are cached for a year as immutable. Everything else,
including HTML documents, is sent with `no-cache`.

### The `headless` feature

`headless` compiles out the embedding and the asset check. API routes behave normally, non-API routes return the plain
404 above, and the server logs a warning at startup. It exists so that Rust work doesn't depend on the frontend toolchain,
and it's for development and checks only: it never ships.

Because a feature that removes functionality is unusual, `--all-features` means no frontend. Tasks avoid that flag. The
embedded configuration gets its own Clippy and test run in `check:embedded`, so that serving code isn't left unchecked.

## Development versus production execution

|                 | `mise run dev`                         | Production (`mise run build:release`) |
| --------------- | -------------------------------------- | ------------------------------------- |
| Browser opens   | Vite, on <http://localhost:5173>       | `miniban`, on <http://127.0.0.1:3000> |
| Frontend        | Served by Vite from `frontend/src`     | Embedded in `miniban`                 |
| `/api` requests | Proxied by Vite to the headless server | Handled by `miniban` directly         |
| Server build    | Debug, with `headless`                 | Release, without `headless`           |
| Runtime needs   | Deno and the source tree               | The executable alone                  |

Both give the browser the same same-origin contract, since Vite's proxy keeps requests relative. The proxy targets
`MINIBAN_HOST` and `MINIBAN_PORT` from the process environment, with the server's defaults. It ignores `frontend/.env*`
files, which the server can't see.

`mise run dev` runs `cargo run -p miniban-server --features headless`, which replaces `target/debug/miniban` with a headless binary until
the next `mise run build:back`. The startup warning makes this visible.

### Plain Cargo

`mise run check` and `mise run test` are the normal route. Plain Cargo also works, and on a fresh clone these forms need
no frontend build:

```sh
cargo clippy --workspace --all-targets --features miniban-server/headless -- -D warnings
cargo test --workspace --features miniban-server/headless
cargo run -p miniban-cli -- --help
```

`cargo run -p miniban-server` needs a prior `mise run build:front`, or `--features headless`.

## `miniban` and `mb`

The two executables are independent programs from one workspace, so they share a version and a lockfile and are meant to
be released together. Their only shared code is `miniban-core`.

`mb` doesn't talk to `miniban` or to any provider: it has no HTTP client, and `--help` and `--version` work with no server
running. Neither executable depends on the other.
