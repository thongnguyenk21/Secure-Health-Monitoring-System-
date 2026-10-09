# Security Policy

> Placeholder — expanded by RW-02/RW-03.

## Reporting a vulnerability
Report privately to the maintainers (contact address to be added). Do not open a
public issue for security-sensitive reports.

## Scope
- Never commit secrets, tokens, private keys or real patient data.
- Model updates are Ed25519-signed and hash-verified (`ADR-0006`).
- CI runs `gitleaks`, `trivy`, `cargo-audit` (see `.github/workflows/security-scan.yml`, RW-02).
