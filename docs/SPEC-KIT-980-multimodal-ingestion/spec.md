# SPEC-KIT-980 — Multi‑Modal Ingestion (PDF/DOCX now; Images/Audio gated)

**Program:** 2026-Q1 Memvid-first Workbench (stretch / parallelizable)
**Status:** COMPLETED
**Completed:** 2026-01-28
**Owner:** Ingestion Lead + Rust Platform
**Depends on:** SPEC-KIT-971 (Capsule Foundation), SPEC-KIT-972 (Retrieval harness)

> **Implementation status:** Canonical completion tracker: `codex-rs/SPEC.md` Completed (Recent).

## Summary

Enable **multi-modal artifact ingestion** into Memvid capsules so Spec‑Kit can search and time‑travel across:

* PDFs (design docs, audits, exported reports),
* DOCX (PRDs, customer requirements),
  with optional power features for images/audio behind compile-time gates.

This turns “memory” into a real workbench substrate (not just code/text logs).

## Decision IDs implemented

**Implemented by this spec:** D11, D37

**Referenced (must remain consistent):** D5, D6

**Explicitly out of scope:** D32

***

## Goals

* Ingest PDFs and DOCX files as first-class artifacts with provenance metadata.
* Make extracted text searchable via Memvid hybrid retrieval.
* Ensure artifacts are queryable **as-of** a checkpoint and included in **run capsule exports** when requested.
* Keep default builds slim (feature-gated extraction).

## Non-Goals

* Building a hosted ingestion service.
* Perfect extraction fidelity for every PDF edge case.
* Always-on image/audio ingestion in the default build (these are gated).

## Deliverables

* `speckit ingest <PATH> [--tags ...] [--spec <SPEC_ID>] [--stage <STAGE>]`
  * Detects file type and routes to the correct extractor.
  * Stores original bytes (for provenance) + extracted text (for retrieval).
* PDF ingestion behind feature gate: `memvid-pdf` / `pdf_extract`
* DOCX ingestion behind feature gate: `memvid-docx` / `docx_extract`
* Retrieval integration:
  * extracted text is indexed into `lex` + `vec` tracks (hybrid)
  * results include canonical URI + source path + checkpoint
* Evaluation harness extension:
  * add golden queries for PDFs/DOCX in `specs/fixtures/`
* Documentation + examples:
  * “How to ingest a PDF audit report”
  * “How to ingest a DOCX PRD”

### Optional (power-user build, not required for Q1 exit)

* Image embeddings (CLIP) behind `memvid-clip`
* Audio transcription (Whisper) behind `memvid-whisper`

## Acceptance Criteria

* Ingesting a sample PDF and DOCX yields searchable hits via `/speckit.search` (Memvid backend).
* Search results include provenance (path, type, spec\_id/stage if provided) and are stable across reopen.
* Disabling `memvid-pdf`/`memvid-docx` compiles successfully and shows a clear UX error when ingest is attempted.
* As-of query: ingestion committed at checkpoint `X` is visible in checkpoint `X` and later, but not before.

## Dependencies

* Capsule ingestion APIs from SPEC-KIT-971 (put + metadata + checkpoint).
* Retrieval API contract from SPEC-KIT-972 (filters, explain scoring).

## Rollout / Rollback

* Rollout behind feature flags + config (`ingestion.pdf=on/off`, `ingestion.docx=on/off`).
* Rollback: disable config or compile without features; no data migration required.

## Risks & Mitigations

* **Native dependency pain** (PDF parsing libs) → keep behind feature gates; provide preflight checks + clear error messages.
* **Extraction quality variance** → store original bytes + extracted text; allow re-extract in future; add fixtures.
* **Binary size bloat** → default build excludes heavy extractors.
