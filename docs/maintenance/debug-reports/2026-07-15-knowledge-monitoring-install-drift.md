# Knowledge Monitoring Install Drift

## Symptom

The installed 光核超级服务 app reported no knowledge even though the governed project source tree contained eligible `metadata.yml` packages.

## Evidence

- The localhost PAISHU service health response was `ok`, but its storage counters were one collection, zero documents, zero chunks.
- `/Applications/光核超级服务.app` reported version `1.5.0`.
- The installed native binary did not contain the `sync_knowledge_sources` command, while the current source did.
- Governed knowledge packages remained present under `~/Desktop/GUANGHE-PAISHU/knowledge-retrieval`.

## Root Cause

The source repository contained the auto-sync repair, but the installed application was an earlier `1.5.0` bundle. The app could read a populated service but had no command capable of repopulating an empty service, so refresh kept returning an empty dashboard. After deployment, a second configuration gate was found: all packages still had `ingestion.enabled: false` from the earlier explicit vector reset. A final data-contract defect used package-local relative paths as global document IDs, so same-named files from different packages overwrote one another. During the migration, an orphaned legacy ingest process continued posting old IDs and later bulk-disabled the canonical records, which explained the transient duplicate and `0 enabled` states after the first repair.

## Repair

- Package the governed auto-sync implementation in desktop version `1.5.1`.
- Enable the seven cleaned `kb_only` packages requested for monitoring while leaving `ima-prompt-library` quarantined.
- Use native loopback API upserts with `<package>/<relative-path>` IDs and manifest SHA-256 deduplication; preserve exact original source URIs.
- Stop the orphaned legacy ingest process, remove only the 39 verified non-canonical duplicates after a database backup, and restore the 39 canonical documents to enabled state through the audited localhost API.
- Release desktop version `2.0.0` with a separate read-only 200-second status monitor. The initial load and explicit refresh button remain the only paths that can perform governed source synchronization.
- Add `open_knowledge_source` to select the exact source file after native UUID and governed-root validation.
- Add `delete_knowledge` as a recoverable archive: move the source into app-local knowledge trash, persist a tombstone, disable retrieval, and roll back on service failure.
- Hide tombstoned documents from returned inventory and aggregate document/chunk counts.

## Verification

- Frontend Knowledge Board tests cover refresh, enable/disable, exact-source reveal, confirmation-gated delete, and overview translation.
- Rust tests cover governed path rejection, source archive and rollback, tombstone loading, adjusted dashboard totals, and the existing service contracts.
- A verified pre-migration PostgreSQL backup was created before existing external IDs were upgraded in place.
- After one full monitoring interval, the service remains stable at 39 unique source documents, 39 enabled, 0 disabled, and 135 active chunks; only the installed `1.5.1` desktop app process remains.
- The installed UI reports `1 个集合 · 39 份知识`, `39 启用 · 0 禁用`, and exposes exact-source reveal, reversible enable/disable, and recoverable delete actions for each row.
- Release packaging, installed binary command presence, and live app restart are verified separately before delivery.

## Version 2.0 Follow-up

- Before the v2.0 release, a read-only database audit found 78 document rows for 39 unique source URIs: 39 canonical `<package>/<relative-path>` IDs and 39 older root-relative IDs containing `/kb/`. The v2.0 app's first and manual governed sync both reported `0` new or updated documents and `39` skips, so the automatic monitor did not create the duplicates.
- A backup-first, count-guarded cleanup removed exactly the 39 legacy records, their 39 revisions, and their 135 associated chunks, then re-enabled the 39 canonical records. Post-cleanup checks report 39 documents, 39 enabled, 0 disabled, 39 unique source URIs, and no root-relative IDs.
- PostgreSQL retains 33 inactive chunks from prior revisions. The service and UI intentionally report the 135 active chunks available to vector retrieval; 168 is the historical physical-row total and is not a dashboard regression.
- The installed `/Applications/光核超级服务.app` is version `2.0.0`; its manual sync regression check returned `0 个新增/更新，39 个跳过` and the visible dashboard remains `39 启用 · 0 禁用 · 135 个分块`.
