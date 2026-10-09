# deploy — Single Deploy Home

- **Purpose:** The one and only home for deployment artifacts (compose, k8s, helm, observability, security, secrets). No tier-local `deploy/` directories exist.
- **Tier:** `[PLAT]` (platform / DevOps)
- **Scope:** `[MVP]` core (compose.dev, compose.pi3, k8s/helm edge+cloud, prometheus/grafana, dev certs); `[LATER]` observability (loki/otel), policy engines, sealed-secrets.
- **Owner:** RW-10 (MVP-07)
- **Tech:** Docker Compose, K3s/Kubernetes, Helm, Prometheus/Grafana, SOPS.
