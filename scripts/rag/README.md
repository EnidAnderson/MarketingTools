# NDOC RAG Scripts

These scripts build machine-readable summaries of Rust code declarations and NDOC structured docstrings.

## Usage
1. Build index:
```bash
python3 scripts/rag/build_ndoc_index.py
```

2. Build detailed summary:
```bash
python3 scripts/rag/summarize_ndoc.py
```

3. Audit coverage:
```bash
python3 scripts/rag/audit_ndoc_coverage.py
```

## Outputs
1. `planning/reports/ndoc_index.json`
2. `planning/reports/ndoc_summary.md`
3. `planning/reports/ndoc_coverage.md`

## Notes
1. Scripts currently scan:
- `rustBotNetwork/app_core/src`
- `src-tauri/src`
2. NDOC format is defined in `planning/DOCSTRING_STANDARD.md`.
