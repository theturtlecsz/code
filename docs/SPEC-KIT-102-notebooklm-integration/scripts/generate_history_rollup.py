#!/usr/bin/env python3
"""
SPEC-KIT-102: History Rollup Generator (Seeding Script)

Generates HISTORY_ROLLUP.md from local-memory SQLite database for manual
NotebookLM ingestion. This is a FALLBACK option - the preferred approach
is Dynamic Context Compilation (see spec.md Section 3.2.1).

Usage:
    python3 generate_history_rollup.py [--db-path PATH] [--output PATH] [--min-importance N]

Environment Variables:
    LM_DB_PATH: Override default database path

Default paths based on LOCAL-MEMORY-ENVIRONMENT.md documentation.
"""

import sqlite3
import datetime
import textwrap
import os
import argparse
import hashlib
import json
from pathlib import Path
from typing import Optional, List, Dict, Any

# Configuration defaults
DEFAULT_DB_PATH = Path.home() / ".local-memory" / "unified-memories.db"
DEFAULT_OUTPUT = "HISTORY_ROLLUP.md"
DEFAULT_MIN_IMPORTANCE = 8
MAX_MEMORIES = 500  # Safety limit to prevent massive files


def get_db_path(override: Optional[str] = None) -> Path:
    """Determine database path with priority: CLI arg > env var > default."""
    if override:
        return Path(override)
    if "LM_DB_PATH" in os.environ:
        return Path(os.environ["LM_DB_PATH"])
    return DEFAULT_DB_PATH


def fetch_memories(
    db_path: Path,
    min_importance: int = DEFAULT_MIN_IMPORTANCE,
    limit: int = MAX_MEMORIES
) -> Optional[List[Dict[str, Any]]]:
    """
    Connect to SQLite and fetch high-importance memories chronologically.

    Returns list of memory dicts or None on error.
    """
    print(f"Connecting to database: {db_path}")

    if not db_path.exists():
        print(f"ERROR: Database not found at {db_path}")
        print("Verify path or set LM_DB_PATH environment variable.")
        return None

    try:
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        # Query based on schema from LOCAL-MEMORY-ENVIRONMENT.md
        # Note: Using COALESCE for potentially missing columns
        query = """
            SELECT
                id,
                created_at,
                COALESCE(importance, 5) as importance,
                content,
                COALESCE(tags, '') as tags,
                COALESCE(domain, 'unknown') as domain,
                COALESCE(session_id, 'unknown') as session_id,
                COALESCE(source, 'unknown') as source
            FROM
                memories
            WHERE
                COALESCE(importance, 5) >= ?
            ORDER BY
                created_at ASC
            LIMIT ?
        """
        cursor.execute(query, (min_importance, limit))
        rows = cursor.fetchall()
        conn.close()

        return [dict(row) for row in rows]

    except sqlite3.Error as e:
        print(f"Database error: {e}")
        return None


def parse_timestamp(ts: str) -> str:
    """
    Parse timestamp with fallback for various formats.
    Returns formatted string or original on failure.
    """
    if not ts:
        return "Unknown date"

    formats = [
        "%Y-%m-%dT%H:%M:%S.%fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S.%f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S.%f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d",
    ]

    for fmt in formats:
        try:
            dt = datetime.datetime.strptime(ts, fmt)
            return dt.strftime('%Y-%m-%d %H:%M:%S')
        except ValueError:
            continue

    return ts  # Return original if parsing fails


def extract_structure(content: str) -> Dict[str, str]:
    """
    Extract structured sections from memory content if present.
    Looks for [TYPE], CONTEXT:, REASONING:, OUTCOME: markers.
    """
    result = {
        "type": "unstructured",
        "context": "",
        "reasoning": "",
        "outcome": "",
        "raw": content
    }

    # Check for type prefix
    if content.startswith("["):
        end_bracket = content.find("]")
        if end_bracket > 0:
            result["type"] = content[1:end_bracket].lower()

    # Extract sections
    sections = {
        "context": "CONTEXT:",
        "reasoning": "REASONING:",
        "outcome": "OUTCOME:"
    }

    for key, marker in sections.items():
        if marker in content:
            start = content.find(marker) + len(marker)
            # Find next section or end
            end = len(content)
            for other_marker in sections.values():
                if other_marker != marker:
                    pos = content.find(other_marker, start)
                    if pos > 0 and pos < end:
                        end = pos
            result[key] = content[start:end].strip()

    return result


def format_memory_as_markdown(memory: Dict[str, Any], index: int) -> str:
    """Format a single memory record into Markdown narrative section."""

    date_str = parse_timestamp(memory.get('created_at', ''))
    importance = memory.get('importance', 5)
    content = memory.get('content', '')
    tags = memory.get('tags', '')
    domain = memory.get('domain', 'unknown')
    session_id = memory.get('session_id', 'unknown')
    source = memory.get('source', 'unknown')

    # Extract structure if present
    structure = extract_structure(content)

    # Format content block
    if structure["type"] != "unstructured":
        content_block = f"""
**Type**: `{structure['type'].upper()}`

**Context**: {structure['context'] or 'Not specified'}

**Reasoning**: {structure['reasoning'] or 'Not specified'}

**Outcome**: {structure['outcome'] or 'Not specified'}
"""
    else:
        # Indent raw content as blockquote
        content_block = textwrap.indent(content.strip(), '> ')

    # Parse tags for better display
    tag_list = [t.strip() for t in tags.split(',') if t.strip()] if tags else []
    tags_display = ', '.join(f'`{t}`' for t in tag_list[:10])  # Limit tags shown
    if len(tag_list) > 10:
        tags_display += f' (+{len(tag_list) - 10} more)'

    md = f"""
### Entry {index}: {date_str}

| Attribute | Value |
|-----------|-------|
| **Importance** | {importance}/10 |
| **Domain** | `{domain}` |
| **Session** | `{session_id[:16]}...` |
| **Source** | `{source}` |
| **Tags** | {tags_display or 'None'} |

{content_block}

---
"""
    return md


def generate_statistics(memories: List[Dict[str, Any]]) -> str:
    """Generate summary statistics for the rollup."""

    if not memories:
        return "No memories to analyze."

    # Domain distribution
    domains: Dict[str, int] = {}
    for m in memories:
        d = m.get('domain', 'unknown')
        domains[d] = domains.get(d, 0) + 1

    # Importance distribution
    importance_dist: Dict[int, int] = {}
    for m in memories:
        imp = m.get('importance', 5)
        importance_dist[imp] = importance_dist.get(imp, 0) + 1

    # Tag frequency (top 20)
    tag_freq: Dict[str, int] = {}
    for m in memories:
        tags = m.get('tags', '')
        if tags:
            for t in tags.split(','):
                t = t.strip()
                if t:
                    tag_freq[t] = tag_freq.get(t, 0) + 1
    top_tags = sorted(tag_freq.items(), key=lambda x: -x[1])[:20]

    stats = f"""
## Statistics Summary

### Domain Distribution
| Domain | Count | Percentage |
|--------|-------|------------|
"""
    for domain, count in sorted(domains.items(), key=lambda x: -x[1]):
        pct = (count / len(memories)) * 100
        stats += f"| `{domain}` | {count} | {pct:.1f}% |\n"

    stats += f"""
### Importance Distribution
| Score | Count |
|-------|-------|
"""
    for imp in sorted(importance_dist.keys(), reverse=True):
        stats += f"| {imp} | {importance_dist[imp]} |\n"

    stats += f"""
### Top Tags (by frequency)
| Tag | Occurrences |
|-----|-------------|
"""
    for tag, count in top_tags:
        stats += f"| `{tag}` | {count} |\n"

    return stats


def generate_rollup(
    memories: List[Dict[str, Any]],
    min_importance: int
) -> str:
    """Generate the full Markdown rollup document."""

    if not memories:
        return f"""# Operational Diary (History Rollup)

> **Generated**: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

No memories found meeting criteria (Importance >= {min_importance}).

Check database path and importance threshold.
"""

    # Calculate content hash for cache validation
    content_for_hash = json.dumps([m.get('id', '') for m in memories], sort_keys=True)
    content_hash = hashlib.sha256(content_for_hash.encode()).hexdigest()[:16]

    header = f"""# Operational Diary: codex-rs (HISTORY_ROLLUP.md)

> **Purpose**: Aggregated narrative timeline for NotebookLM ingestion (Tier 2 Memory Seeding)
> **Generated**: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}
> **Memories Included**: {len(memories)}
> **Importance Threshold**: >= {min_importance}
> **Content Hash**: `{content_hash}`

---

## Overview

This document aggregates high-importance memories from the local-memory store
for ingestion into NotebookLM as part of the SPEC-KIT-102 Tiered Memory Architecture.

**Note**: This is a FALLBACK seeding mechanism. The preferred approach is
Dynamic Context Compilation which generates task-specific context on demand.

---

"""

    # Add statistics
    header += generate_statistics(memories)
    header += "\n---\n\n## Memory Timeline\n\n"

    # Format each memory
    body = ""
    for i, memory in enumerate(memories, 1):
        body += format_memory_as_markdown(memory, i)

    footer = f"""
---

## Document Metadata

- **Generator**: SPEC-KIT-102 Seeding Script
- **Schema Version**: 1.0
- **Compatible With**: NotebookLM (Google)
- **Max Token Estimate**: ~{len(memories) * 500} tokens (rough estimate)

### Usage Instructions

1. Upload this file to NotebookLM as a source document
2. The system will index the content for semantic retrieval
3. Query NotebookLM with questions about project history, patterns, and decisions

### Refresh Policy

Regenerate this document when:
- 50+ new high-importance memories are added
- Major architectural changes occur
- Cache TTL expires (recommended: weekly)
"""

    return header + body + footer


def main():
    parser = argparse.ArgumentParser(
        description="Generate HISTORY_ROLLUP.md for NotebookLM seeding"
    )
    parser.add_argument(
        "--db-path",
        type=str,
        help=f"Path to SQLite database (default: {DEFAULT_DB_PATH})"
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        default=DEFAULT_OUTPUT,
        help=f"Output file path (default: {DEFAULT_OUTPUT})"
    )
    parser.add_argument(
        "--min-importance", "-m",
        type=int,
        default=DEFAULT_MIN_IMPORTANCE,
        help=f"Minimum importance threshold (default: {DEFAULT_MIN_IMPORTANCE})"
    )
    parser.add_argument(
        "--limit", "-l",
        type=int,
        default=MAX_MEMORIES,
        help=f"Maximum memories to include (default: {MAX_MEMORIES})"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show statistics without writing file"
    )

    args = parser.parse_args()

    db_path = get_db_path(args.db_path)
    print(f"SPEC-KIT-102: History Rollup Generator")
    print(f"=" * 50)
    print(f"Database: {db_path}")
    print(f"Min Importance: {args.min_importance}")
    print(f"Max Memories: {args.limit}")
    print()

    memories = fetch_memories(db_path, args.min_importance, args.limit)

    if memories is None:
        print("FAILED: Could not retrieve memories")
        return 1

    print(f"Retrieved {len(memories)} memories meeting criteria")

    if args.dry_run:
        print("\n[DRY RUN] Statistics only:\n")
        print(generate_statistics(memories))
        return 0

    rollup_content = generate_rollup(memories, args.min_importance)

    try:
        output_path = Path(args.output)
        output_path.write_text(rollup_content)
        print(f"\nSUCCESS: Generated {output_path}")
        print(f"File size: {output_path.stat().st_size:,} bytes")
        print(f"\nNext step: Upload {output_path} to NotebookLM")
        return 0
    except IOError as e:
        print(f"ERROR: Failed to write output: {e}")
        return 1


if __name__ == "__main__":
    exit(main())
