# Documentation Archive Pack Format

## Overview

Archive packs consolidate documentation that is no longer actively maintained but must remain recoverable.

## Format

```
archive/docs-pack-YYYYMMDD.tar.zst
├── manifest.json          # Metadata and mapping
└── files/                 # Archived documents (original paths preserved)
    └── docs/
        └── archive/
            └── specs/
                └── SPEC-KIT-*/
```

## Manifest Schema

```json
{
  "version": "1.0",
  "created": "2026-01-21T00:00:00Z",
  "pack_name": "docs-pack-20260121",
  "description": "Archived duplicate specs from docs/archive/specs/",
  "source_commit": "<git-sha>",
  "stats": {
    "total_files": 384,
    "total_lines": 150000,
    "total_bytes": 5000000
  },
  "files": [
    {
      "path": "docs/archive/specs/SPEC-KIT-102/spec.md",
      "sha256": "abc123...",
      "lines": 824,
      "bytes": 32000,
      "category": "archive-candidate",
      "tags": ["spec", "archive"],
      "duplicate_of": "docs/SPEC-KIT-102/spec.md",
      "destination": "archive-only"
    }
  ]
}
```

## Commands

### Create Pack
```bash
./scripts/docs-archive-pack.sh create docs/archive/specs
# Output: archive/docs-pack-20260121.tar.zst
```

### List Pack Contents
```bash
./scripts/docs-archive-pack.sh list archive/docs-pack-20260121.tar.zst
```

### Extract Pack
```bash
./scripts/docs-archive-pack.sh extract archive/docs-pack-20260121.tar.zst [target-dir]
```

### Verify Pack Integrity
```bash
./scripts/docs-archive-pack.sh verify archive/docs-pack-20260121.tar.zst
```

## Retention Policy

| Age | Action |
|-----|--------|
| 0-30 days | Keep uncompressed in archive/ |
| 30-90 days | Compress to .tar.zst |
| 90-180 days | Move to cold storage |
| >180 days | Purge (manifest preserved) |

## Recovery

To restore any archived document:

1. Find pack by date: `ls archive/docs-pack-*.tar.zst`
2. Search manifest: `zstd -d -c archive/docs-pack-*.tar.zst | tar -xOf - manifest.json | jq '.files[] | select(.path | contains("SPEC-KIT-102"))'`
3. Extract specific file: `./scripts/docs-archive-pack.sh extract-file <pack> <path>`

## Verification

Every pack must pass:
- [ ] All sha256 checksums match
- [ ] manifest.json is valid JSON
- [ ] No files outside `files/` directory
- [ ] Total byte count matches manifest
