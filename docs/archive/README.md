# Documentation Archive (Packed)

The historical `docs/archive/` tree has been **packed into a zip archive** to materially reduce repo markdown/file sprawl.

**Pack:** `archive/tree-pack-20260127-docs-archive.zip`

## How to browse

```bash
./scripts/docs-archive-pack.sh list archive/tree-pack-20260127-docs-archive.zip
```

## How to restore (local)

```bash
mkdir -p /tmp/docs-archive-restore
./scripts/docs-archive-pack.sh extract archive/tree-pack-20260127-docs-archive.zip /tmp/docs-archive-restore
rsync -a /tmp/docs-archive-restore/files/docs/archive/ docs/archive/
```

