# Contributing

> Placeholder — expanded by RW-02. The full contributing guide (branch naming,
> Conventional Commits, DoD, review rules) will live here.

## Quick rules
- Branch per work package: `wp-NN/<slug>` (e.g. `wp-00/architecture-restructure`).
- Commits follow the Linux-kernel-style `type(scope): summary` format with a wrapped body.
- Every change stays inside the owning work package's globs (see `04-work-split.md`).
- Contracts change ⇒ update `shared/protocols/` + bump `shared/protocols/VERSION` + ADR if breaking.
- No secrets or PII in any commit.
- Rust/Python/TS changes must pass the verify matrix in `04-work-split.md §5`.
