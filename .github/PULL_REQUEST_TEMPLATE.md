## What and why

<!-- One or two sentences: what changes and the reason. -->

## Links

- Work package / requirement: <!-- WP-NN, MVP-0N, REQ-... -->

## Checklist

- [ ] Base branch is `dev` (or `main` for a release or hotfix), not another feature branch.
- [ ] `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` pass in every touched workspace.
- [ ] Public items have doc comments (see `docs/engineering/CODING_CONVENTIONS.md`).
- [ ] Metrics affected (`MET-NN`) are stated with evidence, or "none".
- [ ] No secrets, tokens or personal data in the diff.
- [ ] Commit messages follow `type(scope): summary` (see `CONTRIBUTING.md`).
