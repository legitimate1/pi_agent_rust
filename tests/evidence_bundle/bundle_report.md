# Unified CI Evidence Bundle

> Generated: 2026-08-07T11:41:06Z
> Git ref: 1d686c39
> CI run: local-20260807T114106Z
> Verdict: **COMPLETE**

## Summary

| Metric | Value |
|--------|-------|
| Total sections | 29 |
| Present | 22 |
| Missing | 7 |
| Invalid | 0 |
| Total artifacts | 99 |
| Total size | 1469.4 KB |
| Required present | 12/12 |

## Conformance (6)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Extension conformance summary | PASS | 1 | 1114 B | `tests/ext_conformance/reports/conformance_summary.json` |
| Conformance baseline | PASS | 1 | 16659 B | `tests/ext_conformance/reports/conformance_baseline.json` |
| Conformance event log | PASS | 1 | 221685 B | `tests/ext_conformance/reports/conformance_events.jsonl` |
| Conformance report (Markdown) | PASS | 1 | 48652 B | `tests/ext_conformance/reports/CONFORMANCE_REPORT.md` |
| Regression gate verdict | MISS | 0 | 0 B | `tests/ext_conformance/reports/regression_verdict.json` |
| Conformance trend data | MISS | 0 | 0 B | `tests/ext_conformance/reports/conformance_trend.jsonl` |

## Diagnostics (8)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Must-pass gate verdict | PASS | 1 | 1286 B | `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json` |
| Must-pass gate event log | PASS | 1 | 62186 B | `tests/ext_conformance/reports/gate/must_pass_events.jsonl` |
| Per-extension failure dossiers | MISS | 0 | 0 B | `tests/ext_conformance/reports/dossiers` |
| Health & regression delta report | PASS | 3 | 82184 B | `tests/ext_conformance/reports/health_delta` |
| Provider compatibility matrix | MISS | 0 | 0 B | `tests/ext_conformance/reports/provider_compat` |
| Sharded extension matrix reports | MISS | 0 | 0 B | `tests/ext_conformance/reports/sharded` |
| Extension journey report | PASS | 1 | 820 B | `tests/ext_conformance/reports/journeys/journey_report.json` |
| Auto-repair summary | PASS | 1 | 38722 B | `tests/ext_conformance/reports/auto_repair_summary.json` |

## E2e (1)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| E2E test results | PASS | 74 | 257962 B | `tests/e2e_results` |

## Quarantine (2)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Quarantine report | PASS | 1 | 518 B | `tests/quarantine_report.json` |
| Quarantine audit trail | PASS | 1 | 0 B | `tests/quarantine_audit.jsonl` |

## Performance (6)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Performance budget summary | PASS | 1 | 4216 B | `tests/perf/reports/budget_summary.json` |
| PERF-3X comparison report | PASS | 1 | 6524 B | `tests/perf/reports/perf_comparison.json` |
| PERF-3X parameter sweeps report | PASS | 1 | 1116 B | `tests/perf/reports/parameter_sweeps.json` |
| PERF-3X stress triage report | PASS | 1 | 5063 B | `tests/perf/reports/stress_triage.json` |
| Extension load-time benchmark | MISS | 0 | 0 B | `tests/ext_conformance/reports/load_time_benchmark.json` |
| PERF-3X lineage coherence contract | PASS | 0 | 0 B | `tests/ext_conformance/reports/gate/must_pass_gate_verdict.json | tests/ext_conformance/reports/conformance_summary.json | tests/perf/reports/stress_triage.json` |

## Security (2)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Security and licensing risk review | PASS | 1 | 89657 B | `tests/ext_conformance/artifacts/RISK_REVIEW.json` |
| Extension provenance verification | PASS | 1 | 146944 B | `tests/ext_conformance/artifacts/PROVENANCE_VERIFICATION.json` |

## Traceability (2)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Requirement-to-test traceability matrix | PASS | 1 | 96776 B | `docs/traceability_matrix.json` |
| High-value suite artifact inventory | PASS | 1 | 11541 B | `docs/evidence/high-value-suite-artifact-inventory.json` |

## Inventory (2)

| Section | Status | Files | Size | Path |
|---------|--------|-------|------|------|
| Extension inventory | MISS | 0 | 0 B | `tests/ext_conformance/reports/inventory.json` |
| Extension inclusion manifest | PASS | 4 | 411062 B | `tests/ext_conformance/reports/inclusion_manifest` |

## Missing / Invalid Sections

- **Regression gate verdict** (missing): File not found
  Path: `tests/ext_conformance/reports/regression_verdict.json`
- **Conformance trend data** (missing): File not found
  Path: `tests/ext_conformance/reports/conformance_trend.jsonl`
- **Per-extension failure dossiers** (missing): Directory not found
  Path: `tests/ext_conformance/reports/dossiers`
- **Provider compatibility matrix** (missing): Directory not found
  Path: `tests/ext_conformance/reports/provider_compat`
- **Sharded extension matrix reports** (missing): Directory not found
  Path: `tests/ext_conformance/reports/sharded`
- **Extension load-time benchmark** (missing): File not found
  Path: `tests/ext_conformance/reports/load_time_benchmark.json`
- **Extension inventory** (missing): File not found
  Path: `tests/ext_conformance/reports/inventory.json`

