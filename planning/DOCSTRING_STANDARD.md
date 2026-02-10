# NDOC Structured Docstring Standard

Last updated: 2026-02-10

## Purpose
Define a machine-readable docstring format for Rust declarations so local RAG/indexing scripts can build reliable architecture summaries.

## Format
Use Rust doc comments (`///`) with an `NDOC` header directly above declarations.

Required fields:
1. `component`
2. `purpose`

Optional fields:
1. `invariants`
2. `inputs`
3. `outputs`
4. `failure_modes`
5. `notes`

## Example
```rust
/// # NDOC
/// component: `pipeline`
/// purpose: Execute a sequential pipeline.
/// invariants:
///   - Step order is deterministic.
///   - Failed step terminates execution.
pub async fn execute_pipeline(...) -> Result<...> { ... }
```

## Rules
1. Keep each field on its own line.
2. Keep invariants as bullet lines prefixed by `///   -`.
3. Place NDOC block immediately above the declaration it describes.
4. Treat `component` as a stable identifier used by scripts and docs.

## Enforcement
Run:
1. `python3 scripts/rag/build_ndoc_index.py`
2. `python3 scripts/rag/summarize_ndoc.py`
3. `python3 scripts/rag/audit_ndoc_coverage.py`
