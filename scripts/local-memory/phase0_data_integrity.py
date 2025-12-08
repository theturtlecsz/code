#!/usr/bin/env python3
"""
SPEC-KIT-102 V1 Phase 0: Data Integrity Script

Fixes:
1. Timestamps: Convert Go verbose format to ISO 8601 UTC with 'Z' suffix
2. Agent type: Infer from tags where possible (all currently 'unknown')
3. Schema: Add new columns required for V1
"""

import sqlite3
import re
from datetime import datetime, timezone, timedelta
import json
import sys

DB_PATH = "/home/thetu/.local-memory/unified-memories.db"

# Agent type vocabulary
ALLOWED_AGENT_TYPES = {'human', 'llm_claude', 'llm_gemini', 'llm_other', 'system', 'unknown'}

def parse_go_timestamp(ts: str) -> str:
    """
    Convert Go verbose timestamp to ISO 8601 UTC with Z suffix.

    Input: "2025-09-25 16:34:53.227315703 -0400 EDT m=+6984.437758214"
    Output: "2025-09-25T20:34:53Z"
    """
    if not ts:
        return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    # Already valid ISO 8601 UTC
    if ts.endswith('Z') and 'T' in ts:
        return ts

    # Remove monotonic clock marker (m=+...)
    ts_clean = re.sub(r'\s*m=[+-][\d.]+', '', ts)

    # Remove timezone abbreviation (EDT, EST, etc.)
    ts_clean = re.sub(r'\s+[A-Z]{2,4}\s*$', '', ts_clean)

    # Try parsing with timezone offset
    # Format: "2025-09-25 16:34:53.227315703 -0400"
    patterns = [
        (r'^(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2})\.?\d*\s*([+-]\d{4})$',
         lambda m: parse_with_offset(m.group(1), m.group(2), m.group(3))),
        (r'^(\d{4}-\d{2}-\d{2})\s+(\d{2}:\d{2}:\d{2})\.?\d*$',
         lambda m: f"{m.group(1)}T{m.group(2)}Z"),  # Assume UTC if no offset
        (r'^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})$',
         lambda m: f"{m.group(1)}T{m.group(2)}Z"),
    ]

    for pattern, handler in patterns:
        match = re.match(pattern, ts_clean.strip())
        if match:
            return handler(match)

    # Fallback: use current time
    print(f"  Warning: Could not parse timestamp '{ts[:50]}...', using current time")
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_with_offset(date_str: str, time_str: str, offset_str: str) -> str:
    """Convert local time with offset to UTC ISO 8601."""
    # Parse offset like -0400 or +0530
    sign = 1 if offset_str[0] == '+' else -1
    hours = int(offset_str[1:3])
    minutes = int(offset_str[3:5])
    offset = timedelta(hours=sign * hours, minutes=sign * minutes)

    # Parse datetime
    dt = datetime.strptime(f"{date_str} {time_str}", "%Y-%m-%d %H:%M:%S")

    # Convert to UTC (subtract offset because offset shows local-UTC)
    utc_dt = dt - offset

    return utc_dt.strftime("%Y-%m-%dT%H:%M:%SZ")


def infer_agent_type(tags: str) -> str:
    """Infer agent type from tags string."""
    if not tags:
        return "unknown"

    t = tags.lower()

    # Check for explicit agent tags
    if "agent:claude" in t or "claude" in t or "llm_claude" in t:
        return "llm_claude"
    if "agent:gemini" in t or "gemini" in t or "llm_gemini" in t:
        return "llm_gemini"
    if "agent:llm" in t or "llm:" in t:
        return "llm_other"
    if "agent:system" in t or "system:" in t:
        return "system"
    if "agent:human" in t or "human:" in t or "user:" in t:
        return "human"

    # Check for model-specific patterns
    if "opus" in t or "sonnet" in t or "haiku" in t:
        return "llm_claude"
    if "gpt" in t or "openai" in t:
        return "llm_other"

    return "unknown"


def run_migrations(conn: sqlite3.Connection, dry_run: bool = False):
    """Run schema migrations."""
    cur = conn.cursor()

    # Check existing columns
    cur.execute("PRAGMA table_info(memories)")
    existing_columns = {row[1] for row in cur.fetchall()}

    migrations = []

    # Add initial_priority if not exists
    if 'initial_priority' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN initial_priority INTEGER")

    # Add usage_count if not exists
    if 'usage_count' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN usage_count INTEGER DEFAULT 0")

    # Add last_accessed_at if not exists
    if 'last_accessed_at' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN last_accessed_at DATETIME")

    # Add dynamic_score if not exists
    if 'dynamic_score' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN dynamic_score FLOAT DEFAULT 0.0")

    # Add structure_status if not exists
    if 'structure_status' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN structure_status TEXT DEFAULT 'unstructured'")

    # Add content_raw if not exists
    if 'content_raw' not in existing_columns:
        migrations.append("ALTER TABLE memories ADD COLUMN content_raw TEXT")

    if dry_run:
        print("\n[DRY RUN] Would execute migrations:")
        for m in migrations:
            print(f"  - {m}")
        return

    for migration in migrations:
        print(f"  Running: {migration}")
        cur.execute(migration)

    conn.commit()
    print(f"  Completed {len(migrations)} column migrations")


def create_cache_tables(conn: sqlite3.Connection, dry_run: bool = False):
    """Create Tier 2 synthesis cache tables."""
    cur = conn.cursor()

    # Check if tables exist
    cur.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='tier2_synthesis_cache'")
    if cur.fetchone():
        print("  tier2_synthesis_cache already exists")
        return

    sql_cache = """
    CREATE TABLE IF NOT EXISTS tier2_synthesis_cache (
        input_hash       TEXT PRIMARY KEY,
        spec_hash        TEXT,
        brief_hash       TEXT,
        synthesis_result TEXT NOT NULL,
        suggested_links  TEXT,
        created_at       DATETIME NOT NULL,
        hit_count        INTEGER DEFAULT 0,
        last_hit_at      DATETIME
    )
    """

    sql_deps = """
    CREATE TABLE IF NOT EXISTS cache_memory_dependencies (
        cache_hash TEXT NOT NULL,
        memory_id  TEXT NOT NULL,
        PRIMARY KEY (cache_hash, memory_id),
        FOREIGN KEY (cache_hash) REFERENCES tier2_synthesis_cache(input_hash) ON DELETE CASCADE,
        FOREIGN KEY (memory_id)  REFERENCES memories(id) ON DELETE CASCADE
    )
    """

    sql_idx = "CREATE INDEX IF NOT EXISTS idx_dependency_memory_id ON cache_memory_dependencies(memory_id)"

    if dry_run:
        print("\n[DRY RUN] Would create cache tables")
        return

    cur.execute(sql_cache)
    cur.execute(sql_deps)
    cur.execute(sql_idx)
    conn.commit()
    print("  Created tier2_synthesis_cache and cache_memory_dependencies tables")


def fix_timestamps(conn: sqlite3.Connection, dry_run: bool = False):
    """Fix all timestamps to ISO 8601 UTC with Z suffix."""
    cur = conn.cursor()

    cur.execute("SELECT id, created_at FROM memories WHERE created_at IS NULL OR created_at NOT LIKE '%Z'")
    rows = cur.fetchall()

    print(f"\n  Found {len(rows)} memories with invalid timestamps")

    if dry_run:
        print("[DRY RUN] Would fix timestamps for these samples:")
        for mem_id, ts in rows[:5]:
            new_ts = parse_go_timestamp(ts)
            print(f"    {ts[:40]}... -> {new_ts}")
        return

    fixed = 0
    for mem_id, ts in rows:
        new_ts = parse_go_timestamp(ts)
        cur.execute("UPDATE memories SET created_at = ? WHERE id = ?", (new_ts, mem_id))
        fixed += 1

    conn.commit()
    print(f"  Fixed {fixed} timestamps")


def fix_agent_types(conn: sqlite3.Connection, dry_run: bool = False):
    """Infer agent types from tags where possible."""
    cur = conn.cursor()

    cur.execute("SELECT id, tags, agent_type FROM memories WHERE agent_type = 'unknown'")
    rows = cur.fetchall()

    print(f"\n  Found {len(rows)} memories with agent_type='unknown'")

    updates = {}
    for mem_id, tags, _ in rows:
        inferred = infer_agent_type(tags)
        if inferred != 'unknown':
            updates[mem_id] = inferred

    print(f"  Can infer better agent_type for {len(updates)} memories")

    if dry_run:
        print("[DRY RUN] Would update agent_type distribution:")
        dist = {}
        for at in updates.values():
            dist[at] = dist.get(at, 0) + 1
        for at, cnt in sorted(dist.items()):
            print(f"    {at}: {cnt}")
        return

    for mem_id, agent_type in updates.items():
        cur.execute("UPDATE memories SET agent_type = ? WHERE id = ?", (agent_type, mem_id))

    conn.commit()
    print(f"  Updated {len(updates)} agent_type values")


def initialize_priority_and_score(conn: sqlite3.Connection, dry_run: bool = False):
    """Copy importance to initial_priority and initialize dynamic_score."""
    cur = conn.cursor()

    if dry_run:
        print("\n[DRY RUN] Would initialize initial_priority and dynamic_score")
        return

    # Copy importance to initial_priority where not set
    cur.execute("""
        UPDATE memories
        SET initial_priority = COALESCE(importance, initial_priority, 5)
        WHERE initial_priority IS NULL
    """)

    # Initialize dynamic_score from initial_priority
    cur.execute("""
        UPDATE memories
        SET dynamic_score = COALESCE(initial_priority, 5) / 10.0
        WHERE dynamic_score IS NULL OR dynamic_score = 0.0
    """)

    # Create index for dynamic_score if not exists
    cur.execute("CREATE INDEX IF NOT EXISTS idx_dynamic_score ON memories(dynamic_score DESC)")

    conn.commit()
    print("  Initialized initial_priority and dynamic_score")


def validate_phase0(conn: sqlite3.Connection) -> bool:
    """Run validation queries - must all return 0."""
    cur = conn.cursor()

    print("\n=== Phase 0 Validation ===")

    # Check timestamps
    cur.execute("SELECT COUNT(*) FROM memories WHERE created_at IS NULL OR created_at NOT LIKE '%Z'")
    invalid_ts = cur.fetchone()[0]
    print(f"  Invalid timestamps: {invalid_ts}")

    # Check agent_type (NULL not allowed, but 'unknown' is fine)
    cur.execute("""
        SELECT COUNT(*) FROM memories
        WHERE agent_type IS NULL
           OR agent_type NOT IN ('human', 'llm_claude', 'llm_gemini', 'llm_other', 'system', 'unknown')
    """)
    invalid_agent = cur.fetchone()[0]
    print(f"  Invalid agent_type: {invalid_agent}")

    # Check initial_priority exists
    cur.execute("SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'initial_priority'")
    has_priority = cur.fetchone()[0]
    print(f"  initial_priority column exists: {has_priority == 1}")

    # Check dynamic_score exists
    cur.execute("SELECT COUNT(*) FROM pragma_table_info('memories') WHERE name = 'dynamic_score'")
    has_score = cur.fetchone()[0]
    print(f"  dynamic_score column exists: {has_score == 1}")

    # Check cache tables exist
    cur.execute("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tier2_synthesis_cache'")
    has_cache = cur.fetchone()[0]
    print(f"  tier2_synthesis_cache exists: {has_cache == 1}")

    passed = (invalid_ts == 0 and invalid_agent == 0 and has_priority == 1 and has_score == 1 and has_cache == 1)
    print(f"\n  Phase 0 {'PASSED' if passed else 'FAILED'}")

    # Show agent_type distribution
    print("\n  Agent type distribution:")
    cur.execute("SELECT agent_type, COUNT(*) FROM memories GROUP BY agent_type ORDER BY COUNT(*) DESC")
    for at, cnt in cur.fetchall():
        print(f"    {at}: {cnt}")

    return passed


def main():
    dry_run = "--dry-run" in sys.argv
    validate_only = "--validate" in sys.argv

    if dry_run:
        print("=== DRY RUN MODE ===\n")

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA foreign_keys = ON")

    try:
        if validate_only:
            validate_phase0(conn)
            return

        print("=== Phase 0: Data Integrity ===\n")

        # Step 1: Schema migrations
        print("1. Running schema migrations...")
        run_migrations(conn, dry_run)

        # Step 2: Create cache tables
        print("\n2. Creating cache tables...")
        create_cache_tables(conn, dry_run)

        # Step 3: Fix timestamps
        print("\n3. Fixing timestamps...")
        fix_timestamps(conn, dry_run)

        # Step 4: Infer agent types
        print("\n4. Inferring agent types from tags...")
        fix_agent_types(conn, dry_run)

        # Step 5: Initialize priority and score
        print("\n5. Initializing initial_priority and dynamic_score...")
        initialize_priority_and_score(conn, dry_run)

        # Step 6: Validate
        if not dry_run:
            validate_phase0(conn)

    finally:
        conn.close()


if __name__ == "__main__":
    main()
