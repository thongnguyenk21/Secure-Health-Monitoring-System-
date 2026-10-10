# Coding & Documentation Conventions

Status: normative (enforced in CI). Scope: all source, contracts, docs, and CI in
this repository. If a rule here conflicts with a linter config, the linter config wins
and this file MUST be updated in the same PR.

Audience: engineers.

---

## 1. Language policy

All repository artifacts MUST be written in **English**:

- Source comments and documentation comments.
- Identifiers (types, functions, variables, modules, crates, files).
- Documentation, specs, ADRs, PR/issue text, commit messages, branch names.
- Error messages, log messages, and user-visible strings in code.

The **only** permitted exception is the human-facing C4 page (the published
`arch/**/iomt-c4` artifact), which MAY be in Vietnamese. Everything else —
including that page's source comments — stays English.

Rationale: one searchable vocabulary, no mixed-script diffs, and tooling
(linkers, codegen, log search) that assumes ASCII identifiers.

---

## 2. Documentation comment standard

Every **public item** (module, type, trait, function, field, enum variant,
constant, exported symbol) MUST have a documentation comment with:

1. A **one-sentence summary line** (imperative for functions: "Parse the frame
   header."; noun phrase for types: "A decoded IMU sample at device resolution.").
2. A **blank line**, then optional longer description (invariants, units, ranges).

A missing summary on a public item is a **CI failure**. Do not restate the name;
say what the caller needs to know.

### 2.1 Rust — rustdoc (`///`, module docs `//!`)

```rust
//! Signal-processing pipeline for the edge-lite tier.
//!
//! Consumes frames from `iomt.telemetry.v1` and emits windowed features.

/// Estimate heart rate from a band-pass-filtered PPG window.
///
/// Uses a zero-crossing estimator; results are smoothed over 5 windows.
///
/// # Arguments
/// * `samples` — PPG samples in **mV**, sampling rate [`SAMPLE_HZ`] **Hz**.
/// * `window_ms` — analysis window length in **milliseconds**.
///
/// # Returns
/// Estimated heart rate in **beats per minute** (bpm), or `None` if the signal
/// is flat. Valid range: 30–220 bpm.
///
/// # Errors
/// Returns [`Error::TooShort`] when `samples.len()` covers less than `window_ms`.
///
/// # Panics
/// Panics if `window_ms == 0`.
///
/// # Safety
/// Not applicable (safe function). For `unsafe fn`, this section is **mandatory**
/// and MUST list each caller obligation and the reason the operation is sound.
///
/// # Examples
/// ```
/// let bpm = estimate_hr(&samples, 5_000)?;
/// assert!((30..=220).contains(&bpm.unwrap()));
/// # Ok::<(), iomt_signal::Error>(())
/// ```
pub fn estimate_hr(samples: &[i32], window_ms: u32) -> Result<Option<u8>, Error> {
    // ...
}
```

Sections by kind: `# Arguments` (one bullet per parameter), `# Returns`,
`# Errors` (one line per error variant/condition), `# Panics`, `# Safety`
(mandatory on every `unsafe` item), `# Examples` (a real doctest, compiled in CI).

For **`no_std` firmware crates** the same rules apply, and docs MUST additionally
state hardware assumptions: physical units, timing/isolation constraints, ISR
safety, and register/peripheral dependencies.

CI enforces:

```rust
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::missing_safety_doc)]
```

### 2.2 Python — PEP 257 + Google style

Full type hints on every public signature; never mix a type hint with a
redundant `:type:` docstring field.

```python
def window_rms(samples: list[float], window_ms: int, sample_hz: int) -> list[float]:
    """Compute sliding-window RMS energy over a signal.

    Args:
        samples: Input amplitudes in millivolts (mV), equally spaced.
        window_ms: Window length in milliseconds; must be > 0.
        sample_hz: Sampling rate in hertz (Hz); must be > 0.

    Returns:
        Root-mean-square values, one per non-overlapping window.

    Raises:
        ValueError: If ``window_ms`` or ``sample_hz`` is not positive.

    Yields:
        (generator functions only) One windowed value per iteration.

    Example:
        >>> window_rms([1.0, -1.0, 1.0, -1.0], 2, 1)
        [1.0, 1.0]
    """
```

Sections: summary line, `Args:`, `Returns:`, `Raises:`, `Yields:`, `Example:`.
Major sections separated by a blank line. Enforced by `ruff` (`D` rules,
`pydocstyle` google convention) plus `mypy`/`pyright` for types.

### 2.3 TypeScript / React Native — TSDoc

```ts
/**
 * Queue a telemetry batch for upload.
 *
 * @param batch - Frames to send; max 256 entries to bound memory.
 * @returns A promise resolving to the server-assigned batch id.
 * @throws {@link UploadError} When the device is offline after 3 retries.
 * @example
 * ```ts
 * await uploadBatch(frames);
 * ```
 * @remarks
 * Retries use exponential backoff capped at 30 s. Requires an authenticated session.
 */
export async function uploadBatch(batch: Frame[]): Promise<string> { /* ... */ }
```

Tags: `@param name - description`, `@returns`, `@throws`, `@example`,
`@remarks`. Enforced by `eslint-plugin-tsdoc` (syntax) and `eslint-plugin-jsdoc`
(coverage of public exports).

### 2.4 Protobuf / OpenAPI / SQL

Comment **every** message, field, endpoint, table, and column. Comments MUST
state physical **units** and valid **ranges** where a value is physical.

```proto
// Telemetry sample produced by a body-worn sensor.
message ImuSample {
  // Sample timestamp as Unix epoch milliseconds (UTC).
  int64 ts_unix_ms = 1;
  // Acceleration on the X axis, in milli-g (mg). Valid range: [-8000, 8000].
  int32 accel_x_mg = 2;
}
```

- Protobuf: `//` comment above each message/field; proto package
  `iomt.<domain>.v<major>`.
- OpenAPI: summary + description on each path/operation and each schema property;
  state units and bounds.
- SQL: `COMMENT ON`/inline comment per table and column; document enum domains
  and FK cardinality.

---

## 3. Inline comment rules

- Explain **WHY**, not WHAT. The code already says what.
- Every physical quantity carries its **unit** in code and comment: `Hz`, `ms`,
  `mV`, `bpm`, `°/s` (write `accel_mg`, `timeout_ms`, `sample_hz`).
- TODOs use the exact form `TODO(owner, WP-id): description`.
  Example: `// TODO(alice, WP-04): replace stub after BLE calibration.`
- **No commented-out code.** Delete it; git history is the archive.
- Comments MUST be English (see §1).

---

## 4. Naming, errors, logging, unsafe

### 4.1 Naming (repo conventions)

| Context | Convention | Example |
|---|---|---|
| Directory / doc file | lowercase kebab-case | `signal-processor/`, `ota-rollback.md` |
| Rust crate (package) | kebab-case, shared prefix `iomt-` | `iomt-protocol` |
| Rust module/file | snake_case | `frame_decoder.rs` |
| Rust type/trait | PascalCase; traits by behavior | `FrameDecoder`, `Verify` |
| Python package/module | snake_case | `edge_ml`, `local_trainer` |
| TypeScript file / component | camelCase / PascalCase | `uploadBatch.ts`, `TelemetryCard.tsx` |
| Proto package | `iomt.<domain>.v<major>` | `iomt.telemetry.v1` |
| Git branch | `wp-NN/<slug>` | `wp-04/ble-ingest` |

### 4.2 Error handling

- **Rust libraries:** define typed errors with `thiserror`. **Binaries only**
  MAY use `anyhow`.
- No `unwrap()` / `expect()` in non-test code. Use `?`, `ok_or`,
  `context(...)`, or handle explicitly. Parsing untrusted input MUST return
  `Result`.
- Python: raise specific exception types (never bare `Exception`); no bare
  `except:`. TypeScript: throw `Error` subclasses; do not throw strings.

### 4.3 Logging

- Structured logging via the `tracing` crate (Rust); JSON or key-value fields,
  never string interpolation of values.
- **No PII** and no secrets/tokens in logs (IDs must be pseudonymized).
- Include units in field names (`latency_ms`, `size_bytes`).

### 4.4 Unsafe policy

- `unsafe` is a last resort; document the safety argument in `# Safety`.
- Each `unsafe` block MUST carry a `// SAFETY:` comment stating the invariant
  that makes it sound.
- Firmware ISR/hardware code MUST document shared-state and reentrancy
  assumptions.

---

## 5. Git

- **Commits:** Linux-kernel-style, English, imperative. Subject
  `type(scope): summary` (<= 72 chars, lowercase, no trailing period), blank line,
  body wrapped at 72 columns explaining WHY, optional trailers (`Refs: WP-04`,
  `Fixes: <sha> ("subject")`). Types: feat, fix, docs, refactor, perf, test, build, ci,
  chore, revert. Scope is the tier/module (`fw`, `edge`, `cloud`, `fl`, `mobile`,
  `contracts`, `deploy`, `ci`, `docs`, `adr`, `tools`, `repo`).
  One logical change per commit. No tool attribution trailers.
- **Branches:** `wp-NN/<slug>` (e.g. `wp-04/ble-ingest`).
- **PR checklist** (all boxes required before review):
  - [ ] Links the `REQ-…` / `WP-…` this change satisfies.
  - [ ] Docs and docstrings updated for changed public items.
  - [ ] Tests added/updated and passing; no `unwrap()` in non-test code.
  - [ ] MET impact stated (any `MET-NN` affected, with evidence) or "none".

---

## 6. Definition of Done — documentation

A change is Done only when **all** hold:

- [ ] Every new/changed public item has a summary line + required sections.
- [ ] Every physical value in code/docs has a unit and, where relevant, a range.
- [ ] New modules/tables/endpoints/messages are documented with units.
- [ ] No non-English comments or identifiers were introduced.
- [ ] No commented-out code; TODOs use `TODO(owner, WP-id):`.
- [ ] `docs/` index and any affected spec/ADR updated.

Exact CI commands that enforce this:

```bash
# Rust: docs and lints are errors
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Python: docstrings (Google) + typing
ruff check --select D --line-length 100 .
mypy .            # or: pyright

# TypeScript / React Native
npx tsc --noEmit
npx eslint . --ext .ts,.tsx
```

CI treats warnings as failures; a green build with `-D warnings` is the gate.

---

## Summary

1. All artifacts are English except the human-facing C4 page; every public item carries a summary line plus language-specific sections, enforced by rustdoc/clippy/ruff/tsdoc with warnings as errors.
2. Inline comments explain why, every physical value carries a unit, TODOs use `TODO(owner, WP-id):`, and naming/error/logging/unsafe rules are fixed per language.
3. Git uses Conventional Commits and `wp-NN/<slug>` branches; the DoD checklist plus the exact CI commands (cargo doc/clippy, ruff/mypy, tsc/eslint) gate every merge.
