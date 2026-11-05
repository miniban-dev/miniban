# Miniban Agent Guidance

## Commands

Use `mise` to run tasks. See @mise.toml.

## Code Comments

- Code is the ultimate source of truth: make it clear and readable to minimize the need for docs
- Document all modules. Summarize key types and module use there. Be brief.
- Document all module-public types, constants, methods, and functions
- Do not store context in comments
- Use ASCII only for all comments and documentation
- Prefer using sections with headers over lists when text content would wrap extensively

## Code Style

- Rust: use standard naming guidance from https://rust-lang.github.io/api-guidelines/naming.html
- Rust: prefer type system errors over runtime errors in API design
- Rust: Do not use `mod.rs` pattern to organize modules. Example correct structure is `server.rs` and `server/routes.rs`.
- If type system design makes an API overly complicated, seek feedback from user
- Run the format commands after edits are complete

### Copyright Banner

Put this banner at the top of all source files, using the year the file is created and adapting the comment
style to the file type. Place it after any line that must come first, such as a shebang or doctype. Skip JSON,
Markdown, lockfiles, `LICENSE`, and generated files.

```rust
// Copyright (c) YEAR Braden Hitchcock - AGPL-3.0-only (see LICENSE for details)
```

## Testing

Place Rust tests after business logic in a module underneath this comment line:

```rust
// -------------------------------------------------------------------------------------------------
// END OF BUSINESS LOGIC - TESTS BELOW HERE
// -------------------------------------------------------------------------------------------------
```

## Boundaries

- **ALWAYS** run `check`, `test`, and `fmt` mise tasks before finishing a task
- **ALWAYS** ask before adding new dependencies, giving the user the choice, unless already named explicitly in a ticket
- **NEVER** push yourself unless the user asks you to explicitly

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues for miniban-dev/miniban, using the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage roles map onto Miniban's `kind/*`, `state/*`, `work/*`, `role/*`, `exec/*` and `wayfinder/*` label conventions. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
