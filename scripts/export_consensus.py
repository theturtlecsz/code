#!/usr/bin/env python3
"""Export consensus artifacts from SQLite to evidence directory structure"""

import sqlite3
import json
import sys
from pathlib import Path
from datetime import datetime

def main():
    if len(sys.argv) < 2:
        print("Usage: export_consensus.py SPEC-ID")
        sys.exit(1)

    spec_id = sys.argv[1]

    # Paths
    db_path = Path.home() / ".code" / "consensus_artifacts.db"
    script_dir = Path(__file__).parent
    evidence_root = script_dir.parent / "docs" / "SPEC-OPS-004-integrated-coder-hooks" / "evidence"
    consensus_dir = evidence_root / "consensus" / spec_id

    print(f"Exporting consensus evidence for {spec_id}...")
    print(f"  Database: {db_path}")
    print(f"  Target: {consensus_dir}")
    print()

    # Create directory
    consensus_dir.mkdir(parents=True, exist_ok=True)

    # Connect to database
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row

    # Get stages
    cursor = conn.execute("""
        SELECT DISTINCT stage
        FROM consensus_synthesis
        WHERE spec_id = ?
        ORDER BY created_at
    """, (spec_id,))

    stages = [row['stage'] for row in cursor.fetchall()]

    if not stages:
        print(f"⚠️  No consensus synthesis found for {spec_id}")
        sys.exit(1)

    print(f"Found stages: {', '.join(stages)}")
    print()

    # Export each stage
    for stage in stages:
        stage_name = stage.replace('spec-', '')
        print(f"Exporting {stage_name}...")

        # Export synthesis (combined consensus output)
        synthesis_cursor = conn.execute("""
            SELECT *
            FROM consensus_synthesis
            WHERE spec_id = ? AND stage = ?
            ORDER BY created_at DESC
            LIMIT 1
        """, (spec_id, stage))

        synthesis_row = synthesis_cursor.fetchone()
        if synthesis_row:
            synthesis_data = dict(synthesis_row)
            synthesis_file = consensus_dir / f"{stage_name}_synthesis.json"
            with open(synthesis_file, 'w') as f:
                json.dump(synthesis_data, f, indent=2)
            print(f"  ✓ {synthesis_file.name}")

        # Export verdict (agent proposals)
        verdict_cursor = conn.execute("""
            SELECT agent_name, content_json, created_at
            FROM consensus_artifacts
            WHERE spec_id = ? AND stage = ?
            ORDER BY created_at
        """, (spec_id, stage))

        proposals = []
        for row in verdict_cursor.fetchall():
            try:
                content = json.loads(row['content_json'])
            except:
                content = row['content_json']

            proposals.append({
                'agent_name': row['agent_name'],
                'content': content,
                'created_at': row['created_at']
            })

        verdict_data = {
            'spec_id': spec_id,
            'stage': stage_name,
            'proposals': proposals,
            'exported_at': datetime.utcnow().isoformat() + 'Z'
        }

        verdict_file = consensus_dir / f"{stage_name}_verdict.json"
        with open(verdict_file, 'w') as f:
            json.dump(verdict_data, f, indent=2)
        print(f"  ✓ {verdict_file.name}")

    conn.close()

    print()
    print("✅ Consensus evidence exported successfully")
    print()
    print("Files created:")
    for f in sorted(consensus_dir.glob('*.json')):
        size_kb = f.stat().st_size / 1024
        print(f"  {f.name:30s} {size_kb:8.1f} KB")

if __name__ == '__main__':
    main()
