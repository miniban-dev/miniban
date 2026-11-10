# Research: ConnectRPC for the server and web UI contract

Ticket: [#15](https://github.com/miniban-dev/miniban/issues/15), for the
interface contract decision in map
[#11](https://github.com/miniban-dev/miniban/issues/11). Researched 2026-11-09.

## Answer

**ConnectRPC fits, with no blockers.** The official Rust implementation,
`connectrpc` 0.9.1, mounts on the existing Axum 0.8 router under `/api`, next to
`/api/health` and the embedded SPA fallback. The TypeScript client,
`@connectrpc/connect-web` 2.3.0, builds and type-checks in the existing
SvelteKit, Deno and Vite frontend. A throwaway spike confirmed both, end to end:
unary JSON over HTTP/1.1, Connect GET, errors, and server streaming through the
Vite dev proxy, all callable with plain `curl`.

The risks are maturity and toolchain weight, not fit:

- The Rust side is pre-1.0, about eight months old, and has one listed
  maintainer.
- It adds a protobuf schema, `buf`, two code generators, and a mapping layer
  between protobuf messages and the domain types.

## How this was researched

There are two kinds of evidence:

- **Sources:** primary sources only. These are the connectrpc.com protocol spec
  and docs, the `connectrpc/connect-rust`, `anthropics/buffa`,
  `connectrpc/connect-es` and `bufbuild/protobuf-es` repositories, and the
  crates.io and npm registry APIs, all read on 2026-11-09.
- **Spike:** a throwaway spike in a scratch directory outside the repo. It used
  the repo's pinned tools: Rust 1.99.1, Deno 2.9.8, Axum 0.8.9, and a copy of
  `frontend/`. Nothing from it is committed.

Each section separates **Verified** facts, which come from a cited source or
were observed in the spike, from **Inference**.

## Rust server support

### Verified

The crates that implement Connect for Rust:

| Crate               | Latest | Released   | Notes                           |
| ------------------- | ------ | ---------- | ------------------------------- |
| `connectrpc`        | 0.9.1  | 2026-09-21 | Official, `connectrpc` org      |
| `connectrpc-axum`   | 0.2.3  | 2026-08-05 | Third party, MIT, 62k dl        |
| `axum-connect`      | 0.5.3  | 2025-06-13 | Third party, idle for 17 months |
| `connect-rpc`       | 0.1.0  | 2024-10-05 | Abandoned                       |
| `arcbox-connectrpc` | -      | -          | Fork; defers to the original    |

Sources: the crates.io API (`https://crates.io/api/v1/crates?q=connectrpc`,
`?q=axum-connect`, `?q=connect-rpc`).

`connectrpc` is the only serious candidate:

- **Status:** Anthropic built it and contributed it to the Connect project. Buf
  announced it as an official implementation on 2026-08-26
  ([Buf blog](https://buf.build/blog/connect-rust-joins-the-connect-project)).
  The repository is
  [connectrpc/connect-rust](https://github.com/connectrpc/connect-rust): Apache
  2.0, 539 stars, last push 2026-11-02, and its README says "pre-1.0. The API
  surface is settling but may shift in 0.x."
- **Maintenance:** `MAINTAINERS.md` lists one maintainer, Iain McGinniss of
  Anthropic. He has 154 commits, and the next most active contributor has 14
  (GitHub contributors API).
- **Release cadence:** 0.5.0 came out on 2026-05-18, 0.6.0 on 2026-05-20, 0.7.0
  on 2026-06-10, 0.8.0 on 2026-07-01, 0.9.0 on 2026-08-25, and 0.9.1 and a 0.8.2
  backport on 2026-09-21. Each minor release has been a 0.x breaking bump.
- **Conformance:** the README says it passes the full Connect conformance suite
  for the Connect, gRPC and gRPC-Web protocols: 3,600 server tests and 6,872
  client tests.
- **Protobuf runtime:** messages come from `buffa` 0.9.2
  ([anthropics/buffa](https://github.com/anthropics/buffa), released
  2026-09-03). It is a pure-Rust protobuf runtime that passes the protobuf
  binary and JSON conformance suites.
- **Rust and Axum versions:** MSRV is Rust 1.88, and the workspace's Rust 1.99.1
  satisfies it. It depends on `axum = "0.8"`, `tower = "0.5"`, `hyper = "1.8"`
  and `http = "1"`, which are the same major versions this workspace uses
  (`Cargo.toml` in the repository).

Axum integration (from the
[README](https://github.com/connectrpc/connect-rust#with-axum-recommended) and
the
[user guide](https://github.com/connectrpc/connect-rust/blob/main/docs/guide.md#hosting)):

- The `axum` feature adds `Router::into_axum_service()`, which returns a Tower
  service, and `into_axum_router()`, which returns a router to merge.
- The documented pattern mounts it as `fallback_service`. That slot is taken
  here by the SPA fallback, so the spike mounted it under a prefix instead.

Spike results (`nest_service("/rpc", connect.into_axum_service())` inside the
existing nested `/api` router, with the `api_not_found` fallback kept):

- `POST /api/rpc/miniban.v1.BoardService/GetIssue` with `application/json`
  returned 200 and canonical proto3 JSON.
- `GET /api/health` still returned `{"status":"ok"}`. `GET /api/nope` still
  returned the 404 JSON fallback, and non-API paths still reached the frontend
  fallback.
- An unknown RPC under the prefix returned 404 with
  `{"code":"unimplemented","message":"method not found: ..."}`.
- A wrong content type returned 415 with an `accept-post` header listing the
  codecs. A GET on a streaming method returned 405.
- Unknown JSON fields in a request were ignored.
- The same server also answered HTTP/2 with prior knowledge (h2c).

Generated code:

- `protoc-gen-connect-rust` generates a service trait, such as `BoardService`,
  with one `async fn` per RPC. Each takes a `RequestContext` and a zero-copy
  `ServiceRequest<'_, Req>` view and returns `ServiceResult<Resp>`. It also
  generates a typed client, which the `client` feature enables.
- `buffa` generates owned message structs, view types and serde JSON impls.
  Optional message fields use `MessageField<T>`, and enums use `EnumValue<T>`,
  an open enum that preserves unknown values.
- The spike implementation compiled on the first attempt. The only noise was the
  `refining_impl_trait` warning that the guide documents.

The dependency graph:

- The spike's graph is about 100 unique crates.
- Default features pull in `zstd-sys`, which needs a C compiler through `cc`,
  and `flate2`. The `default-features = false` configuration drops them
  ([README feature flags](https://github.com/connectrpc/connect-rust#feature-flags)).

### Inference

- The pre-1.0, single-maintainer status is the main long-term risk. Mitigating
  factors: it sits in the CNCF Connect organization
  ([CNCF](https://www.cncf.io/projects/connect-rpc/)), runs in production at
  Anthropic according to Buf's post, and speaks a stable wire protocol. If the
  crate stalled, the browser client and the schema would survive. Only the Rust
  handler glue would need replacing, for example with `tonic` plus `tonic-web`
  serving gRPC-Web.
- Expect a small migration on each 0.x minor release, roughly monthly so far.
  Pin it exactly.
- `connectrpc-axum` is a reasonable second choice if the official crate stalls.
  It is not worth starting there.

## Code generation toolchain

### Verified

Rust supports two workflows
([README](https://github.com/connectrpc/connect-rust#generate-rust-code)):

- **`build.rs` through `connectrpc-build`:** generates at build time. It shells
  out to `protoc` by default, or to `buf` with `Config::use_buf()`, or it reads
  a precompiled `FileDescriptorSet` with `Config::descriptor_set(path)`, which
  needs neither tool
  ([connectrpc-build source](https://github.com/connectrpc/connect-rust/blob/main/connectrpc-build/src/lib.rs)).
  The spike used it with `protoc` 36.2, which mise installed.
- **`buf generate` with committed output:** runs `protoc-gen-buffa`,
  `protoc-gen-connect-rust` and, optionally, `protoc-gen-buffa-packaging`.
  Releases ship signed per-platform binaries with GitHub attestations, both
  plugins are on crates.io, and both are also BSR remote plugins
  (`buf.build/anthropics/buffa`, `buf.build/connectrpc/rust`).

The `buf` CLI:

- `buf` 1.75.0 (2026-10-22) is in the mise registry as `aqua:bufbuild/buf`. The
  spike installed it with `mise exec buf@1.75.0`.
- `protoc` is also in the mise registry, as
  `aqua:protocolbuffers/protobuf/protoc`.

Plugins through mise's `github:` backend:

- `github:connectrpc/connect-rust@0.9.1` installed and verified its
  attestations, but the binary keeps its versioned file name.
- The buffa release holds two plugins. With a naive `asset_pattern`, mise chose
  the Linux binary on macOS.
- So the `github:` backend works but needs per-tool configuration. The `cargo:`
  backend, which builds from crates.io, is the simpler pin.

TypeScript generation:

- `protoc-gen-es` 2.17.0 (2026-11-05) is the only TypeScript plugin needed in
  v2. Service descriptors are generated into `*_pb.ts`, and there is no separate
  `protoc-gen-connect-es`
  ([connectrpc.com: generating code](https://connectrpc.com/docs/web/generating-code/)).
- The plugin script has a `#!/usr/bin/env node` shebang, and the repo pins no
  Node. The spike instead ran it as a `buf` local plugin through Deno, with
  `local: ["deno", "run", "--allow-read", "--allow-env",
  "npm:@bufbuild/protoc-gen-es@2.17.0"]` and Node removed from `PATH`, and it
  generated working code.

### Inference

- **Recommended shape:** commit the generated code for both languages, and add a
  `mise run gen` task that runs `buf generate`.
  - This keeps the current properties that Cargo never invokes another toolchain
    and that plain `cargo` works on a fresh clone. A `build.rs` calling `protoc`
    or `buf` would break both.
  - CI can check freshness with `buf generate && git diff --exit-code`, and `buf
    lint` and `buf breaking` come with the tool.
- **Alternative:** `connectrpc-build` reading a committed descriptor set from
  `buf build`. It avoids Rust plugin binaries but still commits a generated
  binary file.
- **Plugin pinning:** pin the Rust plugins with mise's `cargo:` backend, or
  invoke `connectrpc-build` from a tiny `xtask`. Run `protoc-gen-es` through
  Deno from the frontend's locked `node_modules`, so `deno.lock` pins it.
- **Avoid BSR remote plugins:** they make generation depend on the network and
  on a third-party service.

## TypeScript client under Deno, Vite and Svelte

### Verified

| Package                   | Latest | Released   | Peer deps           |
| ------------------------- | ------ | ---------- | ------------------- |
| `@connectrpc/connect`     | 2.3.0  | 2026-10-27 | protobuf ^2.7.0     |
| `@connectrpc/connect-web` | 2.3.0  | 2026-10-27 | protobuf, connect   |
| `@bufbuild/protobuf`      | 2.17.0 | 2026-11-05 | none                |
| `@bufbuild/protoc-gen-es` | 2.17.0 | 2026-11-05 | protobuf (dev only) |

Sources: the npm registry (`https://registry.npmjs.org/<name>`). None of them
has runtime dependencies beyond its peers.

The transport and client:

- `createConnectTransport({ baseUrl })` accepts a relative base. The source says
  that for same-origin you should "use `baseUrl: window.location.origin` or
  simply `/`".
- JSON is the default format, and `useHttpGet` is off by default
  ([connect-transport.ts](https://github.com/connectrpc/connect-es/blob/main/packages/connect-web/src/connect-transport.ts)).
- Browsers get unary and server-streaming calls only, not client or bidi
  streaming
  ([connectrpc.com: web getting started](https://connectrpc.com/docs/web/getting-started/)).

The environment options:

- The protobuf-es docs say Deno needs `import_extension=ts`. They also say Vite
  projects with `erasableSyntaxOnly` need `erasable_syntax=true`, which turns
  TypeScript enums into `as const` objects
  ([environments.md](https://github.com/bufbuild/protobuf-es/blob/main/docs/src/content/docs/reference/environments.md),
  [plugin-options.md](https://github.com/bufbuild/protobuf-es/blob/main/docs/src/content/docs/reference/plugin-options.md)).
- `ts_nocheck=true` also exists.

Spike results, on a copy of `frontend/` with the three runtime packages added
with `deno add`:

- `svelte-check` passed with 0 errors. `vite build` and the static adapter
  succeeded.
- `deno lint` failed on generated code: `ban-types` on an empty message `& {}`.
  The generated directory would need excluding from lint.
- SvelteKit 3 rejected the `$lib` alias (`module_removed_lib`), so the spike
  used relative imports. This is unrelated to Connect, but it's worth knowing.
- Bundle cost: the page chunk grew from 0.48 kB to 33.4 kB gzipped (120 kB
  minified). That covers the runtime, JSON reflection, and the
  `google.protobuf.Struct` well-known type.
- For comparison, Connect's own benchmark puts a 1-RPC client at about 41 kB
  compressed, against 71 kB for gRPC-Web
  ([connect-web-bench](https://github.com/connectrpc/connect-es/tree/main/packages/connect-web-bench)).
- Generated messages are plain typed objects (`Message<"pkg.Type"> & {...}`).
  `uint64` maps to `bigint`, `optional string` maps to `string | undefined`,
  enums strip their prefix (`IssueState.OPEN`), and `Struct` maps to
  `JsonObject`.
- In Svelte 5, the results were assigned to `$state` runes and rendered without
  adapters.
- The same generated client also ran under plain `deno run` against the server.

### Inference

- **Svelte ergonomics are fine.** Calls are plain async functions and async
  iterables. Svelte needs no counterpart to the React-only
  `@connectrpc/connect-query`, since a small wrapper or `$state` is enough.
- **Use 32-bit integers where they suffice,** such as issue numbers, or the
  `jstype = JS_NUMBER` option. This avoids `bigint`, which `JSON.stringify`
  rejects.
- **About 30 kB gzipped is acceptable** for a local, loopback tool.

## Protocol fit

### Verified

From the [Connect protocol spec](https://connectrpc.com/docs/protocol/):

- **Requests:** paths are `<arbitrary prefix>/<package>.<Service>/<Method>`.
  Unary calls are `POST`, with `application/json` or `application/proto`.
- **GET:** methods marked `idempotency_level = NO_SIDE_EFFECTS` can also be
  called by `GET`, with `message`, `encoding`, `base64`, `compression` and
  `connect=v1` query parameters.
- **HTTP versions:** only bidi streaming requires HTTP/2. Unary, client
  streaming and server streaming work on HTTP/1.1.
- **Unary errors:** a non-200 status with a JSON body `{code, message?,
  details?}`. For example, `not_found` maps to 404 and `invalid_argument`
  to 400.
- **Streaming:** uses `application/connect+json` with 5-byte envelopes (a flags
  byte and a 4-byte big-endian length). The response is always 200, and errors
  arrive in the final end-of-stream envelope.
- **Headers:** `connect-protocol-version: 1` is recommended, and servers may
  reject requests without it. `connect-timeout-ms` sets deadlines.

connect-rust maps `ConnectError { code, message, details }` onto that wire form
and ships `google.rpc` detail types in `connectrpc-types`
([guide: errors](https://github.com/connectrpc/connect-rust/blob/main/docs/guide.md#errors-and-status-codes)).

Spike results, all with `curl` over HTTP/1.1:

- Unary JSON POST and Connect GET worked.
- An error returned `404 {"code":"not_found","message":"no such issue"}`.
- A server-streaming call, sent as a hand-built envelope with `curl
  --data-binary`, delivered three messages about 300 ms apart and then the
  end-of-stream frame `{}`.
- The JSON output followed proto3 canonical JSON:
  - `uint64` became a string (`"number":"1"`).
  - Enums became names (`"ISSUE_STATE_OPEN"`).
  - Fields with default values were omitted (`seq: 0` became `{}`).
  - Integers inside `Struct` came out as `1.0`.

### Inference

- Unary calls are fully debuggable with `curl -H 'content-type:
  application/json' -d '{...}'`.
- Streaming is debuggable with `buf curl`, but awkward by hand because of the
  binary envelope.
- Server streaming covers later live updates without WebSockets.
- Proto3 JSON quirks, such as quoted 64-bit numbers and omitted defaults, are
  invisible through the generated clients and only matter to hand-written
  tooling.

## Fit with the Vite dev proxy, tests and smoke tests

### Verified

- Through the existing Vite proxy (`server.proxy["/api"]`, unchanged, with
  `MINIBAN_PORT` pointed at the spike), unary RPCs, GET RPCs and server
  streaming all worked.
- Stream frames arrived incrementally through the proxy at 0.0 s, 0.3 s and 0.6
  s, with no buffering.
- The connect-web client running under Deno through the proxy got typed results,
  a `ConnectError` with `Code.NotFound`, and the stream events.

### Inference

- **No proxy change is needed** if RPCs live under `/api`, for example
  `/api/rpc`.
- **In-process router tests** (`router().oneshot(...)` in `app.rs`) should keep
  working, because the Connect router is an ordinary Tower service. Tests can
  POST JSON bodies just as they hit `/api/health` today. Handlers can also be
  unit-tested directly, as the guide shows.
- **Smoke tests** against the release binary can use `curl` with JSON for unary
  RPCs, so no generated client is needed in the smoke harness.
- **The `/api` contract table in `docs/architecture.md` changes slightly.**
  Unknown RPC paths under the prefix return Connect's
  `{"code":"unimplemented",...}` 404, not `{"error":"not found"}`. Other `/api`
  routes keep the current shape. Reconcile this when the contract is decided,
  either by documenting the two error shapes or by moving health into Connect.

## Fit with the `mb` CLI as a second client

### Verified

- `connectrpc` generates a typed Rust client. It is enabled by the `client`
  feature and served by a hyper `HttpClient`, plaintext or TLS
  ([guide: clients](https://github.com/connectrpc/connect-rust/blob/main/docs/guide.md#clients)).
- The `gate_client_feature` codegen option puts generated clients behind a Cargo
  feature, so the server build can drop the client stack.

### Inference

- `mb` could share one generated crate with the server, for example a new
  `miniban-api` crate, and call `miniban` with full type safety. That crate
  would carry `connectrpc`, `buffa` and `serde`, so per `docs/architecture.md`
  it must not be `miniban-core`.
- Scripts and agents can still use `curl` with JSON, so the CLI is not the only
  non-browser client.

## Modelling the domain in protobuf

### Verified

- buffa preserves unknown enum values (`EnumValue<T>`) and unknown fields, and
  represents proto3 `optional` as `Option<T>`
  ([buffa README](https://github.com/anthropics/buffa)).
- protobuf-es represents `optional` as `T | undefined`.
- `google.protobuf.Struct` carried arbitrary provider metadata as plain JSON
  objects on both sides in the spike.

### Inference

- **Enums:** proto3 needs a zero `*_UNSPECIFIED` value. That fits "unknown state
  from the provider" well, but adds a case the domain may not want.
- **Absence:** use proto3 `optional` for real absence, and wrap message fields
  for absence.
- **Open-ended data:** `map<string, string>` or `Struct` for provider metadata,
  and `oneof` for sum types. All map to TOML frontmatter without strain.
- **Keep protobuf messages as wire DTOs.** Keep the domain model as plain Rust
  types in `miniban-core`, which is the only way to respect the "core depends on
  no frameworks" rule. Then map between them in the server or API crate.
  - The mapping is boilerplate, but it also keeps the TOML and Markdown format
    (proposal section 13) independent of the wire schema.
  - This is the main ergonomic cost compared with sharing serde types directly.
- **Nothing found blocks the TOML frontmatter plus Markdown constraint,**
  because protobuf never touches the on-disk format.

## Alternatives, briefly

### Plain JSON plus generated TypeScript types

Verified versions:

- `ts-rs` 12.0.1 (2026-01-31,
  [Aleph-Alpha/ts-rs](https://github.com/Aleph-Alpha/ts-rs)).
- `specta` 1.0.5 stable, with 2.0.0-rc.28 released 2026-11-05 and still a
  release candidate, plus `specta-typescript` 0.0.15
  ([specta-rs/specta](https://github.com/specta-rs/specta)).

Inference:

- **How it works:** derive TypeScript from the serde types, keep hand-written
  Axum routes, and keep the current contract unchanged.
- **Strengths:** the fewest new tools (no `buf`, no `.proto` files), and the
  domain types could be shared directly.
- **Weaknesses:** it generates types only, not clients, routes or an error
  model. Streaming would be hand-built, for example with SSE.
- **When it wins:** this is the fallback if Connect's toolchain weight is judged
  too high.

### OpenAPI with `utoipa`

Verified versions:

- `utoipa` 6.0.0 and `utoipa-axum` 0.3.0 (2026-09-22,
  [juhaku/utoipa](https://github.com/juhaku/utoipa)).
- `openapi-typescript` 7.13.0 and `openapi-fetch` 0.17.0 (2026-02-11).

Inference:

- **How it works:** a REST contract with a generated spec and a typed fetch
  client.
- **Strengths:** a well-trodden REST path.
- **Weaknesses:** two generators across two ecosystems, and the spec is derived
  from annotations rather than being the source of truth. It has no streaming
  story beyond SSE described by hand.
- **Verdict:** heavier than ts-rs and weaker than Connect for this use.

### gRPC-Web

Verified:

- `tonic` and `tonic-web` 0.14.9 (2026-10-29,
  [grpc/grpc-rust](https://github.com/grpc/grpc-rust)).
- `grpc-web` 2.1.1 on npm (2026-08-27).
- connect-rust serves gRPC-Web too.
- Connect's benchmark shows larger gRPC-Web bundles (above).

Inference:

- **Weaknesses:** binary-only on the wire by default, not `curl`-friendly, and a
  heavier client.
- **When it matters:** only as an exit route if connect-rust stalls. The
  `connect-web` package also has a gRPC-Web transport, so the frontend could
  keep its generated code.

## Implications for the decisions

For the interface contract decision:

- **ConnectRPC is viable.** Nothing found rules it out, and the spike proved
  that the contract fits the current stack without changing the proxy, the
  embedding or the routing model. A good choice: mount the Connect routes at
  `/api/rpc` (or similar), keep `/api/health` as plain JSON, use the JSON codec
  and HTTP/1.1, and use server streaming for live updates later.
- **Accept the risks consciously:** connect-rust is pre-1.0 with monthly
  breaking minor releases and a single maintainer. Pin it exactly, and keep
  domain types out of generated code so a swap stays cheap.
- **Decide where generated code lives.** The recommendation is to commit it for
  both Rust and TypeScript, with a `mise run gen` task and a CI freshness check,
  so Cargo stays toolchain-free.
- **Decide the crate boundary.** For example, a new `miniban-api` crate holds
  the `.proto` files and the generated Rust, the server and `mb` depend on it,
  and `miniban-core` stays free of protobuf.
- **Update `docs/architecture.md`'s `/api` contract table** for the Connect
  error shape.
- **Fallback:** if the toolchain weight is unwelcome, use plain JSON routes plus
  `ts-rs`, with SSE for streaming.

Dependencies and tools the maintainer would need to approve:

Cargo:

- `connectrpc` 0.9.1, with the `axum` feature, plus `client` for `mb`, ideally
  without default features
- `buffa` and `buffa-types` 0.9.2, with the `json` feature
- `serde` 1 with `derive`
- `connectrpc-types` 0.9, optional, for error details
- `connectrpc-build` 0.9.0, as a build dependency, only for the `build.rs` route

mise:

- `buf` 1.75.0, as `aqua:bufbuild/buf`
- `protoc-gen-buffa` 0.9.2 and `protoc-gen-connect-rust` 0.9.1, as `cargo:` or
  `github:` tools

npm:

- `@connectrpc/connect` and `@connectrpc/connect-web` 2.3.0
- `@bufbuild/protobuf` 2.17.0
- `@bufbuild/protoc-gen-es` 2.17.0, as a dev dependency run through Deno
