# Contributing

## Branching model (Git Flow)

| Branch | Purpose | Created from | Merged into |
|---|---|---|---|
| `main` | Production-ready history. Every commit on it is a release, tagged `vX.Y.Z`. | n/a | n/a |
| `dev` | Integration branch. All finished features land here first. | `main` | `release/*` |
| `feature/<scope>-<slug>` | One feature or work-package slice. | `dev` | `dev` (pull request) |
| `release/<x.y.z>` | Stabilisation of a version: fixes, version bump, docs only. | `dev` | `main` and back into `dev` |
| `hotfix/<slug>` | Urgent fix of a released version. | `main` | `main` and `dev` |

Rules:
- Nobody commits directly to `main` or `dev`. Every change reaches them through a pull request.
- Feature branches are short-lived and small: one logical change, up to a few commits.
- Name features `feature/<scope>-<slug>` with the same scope vocabulary as commits, for example
  `feature/contracts-domain-types`, `feature/fw-offline-buffer`, `feature/edge-sync-queue`.
- `main` is the only production branch (there is no `master`).
- Use merge commits (no fast-forward, no squash) for `feature -> dev` and `release -> main`
  so the branch history stays visible.

## Day-to-day flow

```bash
git switch dev && git pull                      # start from the latest integration branch
git switch -c feature/edge-sync-queue           # one branch per feature
# ... edit, then run the checks of the touched workspace ...
git add <explicit paths>
git commit                                      # see commit format below
git push -u origin feature/edge-sync-queue
# open a pull request: base = dev, compare = feature/edge-sync-queue
```

After the pull request is approved and CI is green, merge it into `dev` with a merge commit and delete the
feature branch.

## Releasing

```bash
git switch dev && git pull
git switch -c release/0.1.0                     # freeze scope; only fixes and version bump here
# bump versions, update the changelog, final checks
git push -u origin release/0.1.0                # pull request: base = main
# after the merge into main:
git switch main && git pull
git tag -a v0.1.0 -m "Release 0.1.0" && git push origin v0.1.0
# back-merge so dev contains the release fixes (pull request: base = dev, compare = main or release/0.1.0)
```

## Hotfix

```bash
git switch main && git pull
git switch -c hotfix/fix-queue-overflow
# fix + test, then pull requests into main AND dev; tag the new patch version on main
```

## Commits

Linux-kernel style, English, imperative:

```
type(scope): short summary in lowercase, no trailing period (72 characters at most)

Body wrapped at 72 columns. Explain what and why, not how. Mention the
trade-off or consequence a reviewer must know (units, limits, compatibility).

Refs: WP-04, REQ-REL-007
```

Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
Scopes: `fw`, `edge`, `edge-train`, `cloud`, `fl`, `mobile`, `contracts`, `deploy`, `ci`, `docs`, `adr`, `tools`, `repo`.

## Pull request checklist

- [ ] Base branch is `dev` (or `main` for a release or hotfix), never a feature branch.
- [ ] Links the work package or requirement (`WP-NN`, `REQ-...`) it satisfies.
- [ ] Format, lint and tests of every touched workspace pass (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`).
- [ ] Public items have doc comments as required by `docs/engineering/CODING_CONVENTIONS.md`.
- [ ] Any metric affected (`MET-NN`) is stated with evidence, or "none".
- [ ] No secrets, tokens or personal data in the diff.

## Other rules

- Contracts change: update `shared/protocols/`, bump `shared/protocols/VERSION`, add an ADR if it breaks compatibility.
- Keep each change inside the owning work package (see `docs/design/04-work-split.md`).
