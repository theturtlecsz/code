# Local-Memory Environment Capture

> **Generated**: 2025-11-30
> **Purpose**: Complete documentation of local-memory installation for replication/reference

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Version** | 1.1.7 (infrastructure v2.1.0) |
| **Memories** | 1,049 entries |
| **Relationships** | 2,502 graph connections |
| **Quality Score** | 95/100 (EXCELLENT) |
| **Database Size** | 26.6 MB SQLite + Qdrant vectors |
| **Scripts** | 34 active hooks |
| **Services** | Daemon, REST API, Ollama, Qdrant |

---

## 1. Installation Structure

### Directory Layout

```
~/.local-memory/
├── unified-memories.db           # Main SQLite database (26.6 MB)
├── unified-memories.db.bak       # Backup copy (6.6 MB)
├── analytics.db                  # Analytics and metrics (65 KB)
├── archived-memories.db          # Archive database (28 KB)
├── trends.db                     # Time-series trends (16 KB)
├── config.yaml                   # Configuration file
├── license.json                  # License information
├── PROTOCOL.md                   # Single-source-of-truth documentation (31 KB)
├── README.md                     # Quick start guide
├── qdrant                        # Qdrant binary executable (87 MB)
├── .qdrant-initialized           # Initialization marker
├── daemon.log                    # Main daemon activity log (11.2 MB)
├── quality_history.log           # Quality metrics history
├── local-memory.pid              # Daemon process ID
├── qdrant_storage/               # Qdrant vector database storage
├── storage/                      # Extended storage (RAFT state, collections)
├── logs/                         # Service logs directory
├── backups/                      # Automatic backups (gz compressed)
├── reports/                      # Weekly reports (Markdown)
├── bootstrap/                    # Graph bootstrap tracking
├── archive/                      # Archived memories
├── snapshots/                    # Database snapshots
├── visualizations/               # Graph visualizations
├── scripts/                      # Symlinks to hooks (34 scripts)
└── viz/                          # Visualization outputs
```

---

## 2. Configuration

### config.yaml

```yaml
profile: default

# Database Configuration
database:
  path: "/home/thetu/.local-memory/unified-memories.db"
  backup_interval: "24h0m0s"
  max_backups: 7
  auto_migrate: true

# REST API Configuration
rest_api:
  enabled: true
  auto_port: true
  port: 3002
  host: "localhost"
  cors: true

# Session Management
session:
  auto_generate: true
  strategy: "git-directory"

# Logging Configuration
logging:
  level: "info"
  format: "console"

# AI Services (auto-detection enabled)
ollama:
  enabled: true
  auto_detect: true
  base_url: "http://localhost:11434"
  embedding_model: "nomic-embed-text"
  chat_model: "qwen2.5:3b"

qdrant:
  enabled: true
  auto_detect: true
  url: "http://localhost:6333"
```

### Service Endpoints

| Service | URL | Purpose |
|---------|-----|---------|
| REST API | `http://localhost:3002/api/v1` | Primary interface |
| Ollama | `http://localhost:11434` | Embeddings + chat |
| Qdrant | `http://localhost:6333` | Vector storage |

---

## 3. Database Schema

### Core Tables

#### memories
```sql
id              TEXT PRIMARY KEY   -- UUID
content         TEXT               -- Knowledge content
source          TEXT               -- Origin reference
importance      INTEGER            -- 1-10 curation level
tags            TEXT/JSON          -- Array of tag strings
session_id      TEXT               -- Session context
domain          TEXT               -- Knowledge domain
embedding       BLOB               -- FAISS vector embeddings
created_at      DATETIME           -- Creation timestamp
updated_at      DATETIME           -- Last modified
agent_type      TEXT               -- claude/gemini/codex
agent_context   TEXT               -- Agent-specific context
access_scope    TEXT               -- session/global/private
slug            TEXT               -- URL-friendly identifier
```

#### memory_relationships
```sql
id                  TEXT PRIMARY KEY   -- Relationship UUID
source_memory_id    TEXT               -- Source memory
target_memory_id    TEXT               -- Target memory
relationship_type   TEXT               -- references|contradicts|expands|similar|sequential|causes|enables
strength            REAL               -- 0.0-1.0 connection strength
context             TEXT               -- Why connected
auto_generated      BOOLEAN            -- System vs manual
created_at          DATETIME           -- Creation time
```

---

## 4. Hook Scripts (34 Active)

### Location
- **Primary**: `~/.claude/hooks/`
- **Symlinks**: `~/.local-memory/scripts/`

### Core Operations

| Script | Purpose | Usage |
|--------|---------|-------|
| `lm-search.sh` | Boolean search with AI | `lm-search.sh "query" [--limit N] [--domain D]` |
| `lm-dashboard.sh` | Health monitoring | `lm-dashboard.sh [--compact\|--extended]` |
| `lm-doctor.sh` | Prerequisites check | `lm-doctor.sh [--json]` |
| `lm-services.sh` | Service health | `lm-services.sh health` |
| `lm-backup.sh` | Export and rotate | `lm-backup.sh [--rotate N]` |
| `lm-import.sh` | Import formats | `lm-import.sh file.json` |
| `lm-export-obsidian.sh` | Obsidian export | `lm-export-obsidian.sh` |
| `lm-aliases.sh` | Shell aliases | `source lm-aliases.sh` |

### Graph Operations

| Script | Purpose |
|--------|---------|
| `lm-graph.sh` | Multi-hop visualization |
| `lm-api.sh` | Unified REST API wrapper |
| `lm-bootstrap-graph.sh` | Build relationships from embeddings |
| `lm-suggest-links.sh` | AI relationship suggestions |
| `lm-graph-analyze.sh` | Graph quality metrics |
| `lm-cluster.sh` | Semantic clustering |
| `lm-cross-domain.sh` | Cross-domain discovery |

### Quality & Maintenance

| Script | Purpose |
|--------|---------|
| `lm-quality-score.sh` | Memory quality assessment |
| `lm-quality-monitor.sh` | Quality regression detection |
| `lm-consolidate.sh` | Merge duplicates |
| `lm-archive.sh` | Archive old memories |
| `lm-prune.sh` | Intelligent pruning |
| `lm-maintenance.sh` | Combined maintenance |
| `lm-batch.sh` | Bulk operations |

### Analytics & Monitoring

| Script | Purpose |
|--------|---------|
| `lm-collector.sh` | Daily metrics |
| `lm-report.sh` | Weekly reports |
| `lm-trends.sh` | Time-series analytics |
| `lm-alerts.sh` | Alert management |
| `lm-anomaly.sh` | Anomaly detection |
| `lm-benchmark.sh` | Performance testing |

### Session Hooks

| Script | Trigger | Purpose |
|--------|---------|---------|
| `session_start.sh` | Claude Code start | Auto-load context |
| `session_end.sh` | Claude Code end | Store learnings |
| `lm-precommit.sh` | Git pre-commit | Capture git context |

---

## 5. CLI + REST Reference (No MCP)

Local Memory must be used via **CLI + REST only** (do not configure or call it via MCP).

| Task | CLI (preferred) | REST (automation) |
|------|------------------|-------------------|
| Search | `lm search "query" --limit 10 --use_ai` | `GET /api/v1/memories/search?query=...&domain=...&limit=...` |
| Store | `lm remember "WHAT: ...\nWHY: ...\nEVIDENCE: ...\nOUTCOME: ..." --type decision --importance 8` | `POST /api/v1/memories` |
| Get by ID | — | `GET /api/v1/memories/{id}` |
| Update | — | `PUT /api/v1/memories/{id}` |
| Delete | `local-memory forget <id>` | `DELETE /api/v1/memories/{id}` |
| Relationships | `~/.claude/hooks/lm-api.sh link <src> <tgt> causes 0.8 "context"` | `POST /api/v1/relationships` |

---

## 6. Storage Guidelines

### Content Template

```
[PATTERN|DECISION|PROBLEM]: <one-line summary>
CONTEXT: <what triggered this>
EVIDENCE: <commit hashes, line numbers, concrete data>
REASONING: <WHY this approach, what alternatives rejected>
OUTCOME: <measurable result>
PATTERN: <generalizable lesson for future>
Files: <paths:lines affected>
```

### Tag Conventions (Namespaced)

```
spec:<SPEC-ID>          spec:SPEC-KIT-071
type:<category>         type:bug-fix, type:pattern, type:milestone
project:<name>          project:codex-rs
component:<area>        component:routing, component:consensus
stage:<stage>           stage:plan, stage:implement
agent:<name>            agent:claude, agent:gemini
```

### Importance Scale

| Level | Use For | Frequency |
|-------|---------|-----------|
| 10 | Crisis events, breaking discoveries | <5% |
| 9 | Architecture decisions, critical patterns | 10-15% |
| 8 | Important milestones, valuable solutions | 15-20% |
| 7- | DO NOT STORE | 0% |

---

## 7. Services Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Claude Code / MCP Client                  │
└─────────────────────────┬───────────────────────────────────────┘
                          │ MCP Protocol
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    local-memory daemon                           │
│                    http://localhost:3002                         │
├─────────────────────────────────────────────────────────────────┤
│  REST API  │  Session Mgmt  │  Query Engine  │  Graph Engine    │
└──────┬─────┴───────┬────────┴───────┬────────┴────────┬─────────┘
       │             │                │                 │
       ▼             ▼                ▼                 ▼
┌────────────┐ ┌──────────┐ ┌─────────────────┐ ┌─────────────────┐
│  SQLite    │ │  Qdrant  │ │     Ollama      │ │   File System   │
│  26.6 MB   │ │  :6333   │ │     :11434      │ │   ~/.local-     │
│            │ │          │ │                 │ │   memory/       │
│ memories   │ │ vectors  │ │ nomic-embed-txt │ │ backups/        │
│ relations  │ │ indices  │ │ qwen2.5:3b      │ │ reports/        │
└────────────┘ └──────────┘ └─────────────────┘ └─────────────────┘
```

---

## 8. Current Metrics

### System Health
```
Memories:         1,049
Relationships:    2,502
Quality Score:    95/100 (EXCELLENT)
Orphan Ratio:     ~9%
Density:          2.38 edges/node
Diversity:        5 relationship types (100%)
```

### Service Status
```
Daemon:     Running (PID: 6)
REST API:   Healthy (localhost:3002)
Ollama:     Connected (qwen2.5:3b, nomic-embed-text)
Qdrant:     Connected (memory_embeddings collection)
Database:   Up-to-date schema (v6)
```

---

## 9. REST API Endpoints

**Base**: `http://localhost:3002/api/v1`

```
GET  /health                    # Service health
GET  /stats                     # System statistics
GET  /memories/search           # Search memories
POST /memories                  # Create memory
PUT  /memories/{id}             # Update memory
GET  /memories/{id}             # Get by ID
DELETE /memories/{id}           # Delete memory
GET  /memories/{id}/related     # Related memories
GET  /memories/{id}/graph       # Graph view
POST /relationships             # Create relationship
POST /relationships/discover    # AI discovery
GET  /categories                # List categories
GET  /domains                   # List domains
```

---

## 10. Cron Schedule

| Time | Script | Purpose |
|------|--------|---------|
| 2:00 AM daily | `lm-collector.sh` | Metrics collection |
| 3:00 AM Sun | `lm-report.sh` | Weekly report |
| 4:00 AM Sun | `lm-maintenance.sh --weekly` | Maintenance |

Install with: `lm-cron-install.sh --install`

---

## 11. Environment Variables

```bash
# Path setup
export PATH="$PATH:$HOME/.claude/hooks"
export PATH="$PATH:$HOME/.local-memory/scripts"

# Auto-set by Claude Code
CLAUDE_SESSION_ID    # Unique session identifier
CLAUDE_WORKDIR       # Working directory

# Optional overrides
LM_DIR               # Base directory (default: ~/.local-memory)
LM_DB_PATH           # Database path
LM_API_BASE          # REST API base URL
```

---

## 12. Troubleshooting

### Quick Health Check
```bash
~/.claude/hooks/lm-dashboard.sh --compact    # One-line status
~/.claude/hooks/lm-dashboard.sh --extended   # Full health
~/.claude/hooks/lm-doctor.sh                 # System check
~/.claude/hooks/lm-services.sh health        # Service status
```

### Common Issues

| Issue | Solution |
|-------|----------|
| Empty search results | `local-memory status` then test basic search |
| Empty graph | `lm-bootstrap-graph.sh --limit 20` |
| High orphan ratio | `lm-bootstrap-graph.sh --orphans-only` |
| Quality degradation | `lm-maintenance.sh --optimize --all` |

---

## 13. Replication Checklist

### Prerequisites
```bash
[ ] Go 1.21+ installed
[ ] SQLite 3.x installed
[ ] Ollama service running (localhost:11434)
[ ] Qdrant service running (localhost:6333)
[ ] curl and jq installed
```

### Critical Files to Copy
```
~/.local-memory/config.yaml
~/.local-memory/unified-memories.db
~/.local-memory/qdrant
~/.local-memory/PROTOCOL.md
~/.claude/hooks/lm-*.sh (all scripts)
```

### Installation Steps
```bash
[ ] Create ~/.local-memory/ directory
[ ] Copy config.yaml
[ ] Copy qdrant binary
[ ] Initialize Qdrant collections
[ ] Copy unified-memories.db (or start fresh)
[ ] Copy ~/.claude/hooks/ scripts
[ ] Create symlinks in ~/.local-memory/scripts/
[ ] Set up cron jobs: lm-cron-install.sh --install
[ ] Run health check: lm-doctor.sh
```

---

## 14. Quick Reference Commands

```bash
# Search
~/.claude/hooks/lm-search.sh "query" --limit 5

# Health
~/.claude/hooks/lm-dashboard.sh --compact

# Graph operations
~/.claude/hooks/lm-api.sh related <id> 10
~/.claude/hooks/lm-api.sh discover <id> 5 0.7
~/.claude/hooks/lm-api.sh link <src> <tgt> references 0.9

# Maintenance
~/.claude/hooks/lm-maintenance.sh --daily
~/.claude/hooks/lm-maintenance.sh --weekly

# Store (CLI)
lm remember "[PATTERN]: Your insight\nEVIDENCE: ...\nOUTCOME: ..." \
  --type pattern \
  --importance 8 \
  --tags "spec:SPEC-KIT-XXX"

# Store (REST)
curl -sS -X POST "http://localhost:3002/api/v1/memories" \
  -H "Content-Type: application/json" \
  -d '{"content":"[PATTERN]: Your insight","importance":8,"domain":"spec-kit","tags":["type:pattern","spec:SPEC-KIT-XXX"]}'
```

---

*Document generated for environment replication and reference.*
