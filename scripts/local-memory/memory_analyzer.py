import sqlite3
import pandas as pd
import json
import os
import sys
import datetime
from collections import Counter
import textwrap

# --- Configuration and Utility Functions ---

def get_db_path():
    """Determines the database path, checking environment variables and common locations."""
    # 1. Check for environment variable LM_DB_PATH
    if "LM_DB_PATH" in os.environ and os.path.exists(os.environ["LM_DB_PATH"]):
        return os.environ["LM_DB_PATH"]
    
    # 2. Check common locations based on documentation
    paths_to_try = [
        os.path.expanduser("~/.local-memory/unified-memories.db"),
        "/home/thetu/.local-memory/unified-memories.db"
    ]
    for path in paths_to_try:
        if os.path.exists(path):
            return path
            
    return None

def safe_parse_tags(tag_string):
    """Safely parses the JSON string array of tags, with fallback for plain text."""
    if not tag_string:
        return []
    try:
        # Attempt JSON parsing first (as per documentation: TEXT/JSON)
        tags = json.loads(tag_string)
        if isinstance(tags, list):
            return [str(tag) for tag in tags]
        return []
    except (json.JSONDecodeError, TypeError):
        # Fallback: assume comma-separated strings if JSON fails or if input is not a string
        return [tag.strip() for tag in str(tag_string).split(',') if tag.strip()]

# --- Data Loading and Preprocessing ---

def load_data(db_path):
    """Connects to the SQLite DB and loads core tables into Pandas DataFrames."""
    print(f"Attempting to analyze database at: {db_path}", file=sys.stderr)

    try:
        conn = sqlite3.connect(db_path)
        
        # Load Memories
        try:
            # Selecting specific columns based on the documented schema
            query = """
                SELECT id, created_at, importance, content, tags, domain, agent_type 
                FROM memories
            """
            df_memories = pd.read_sql_query(query, conn)
        except pd.errors.DatabaseError:
            print("Error: 'memories' table not found.", file=sys.stderr)
            return None, None

        # Load Relationships
        try:
            query = """
                SELECT source_memory_id, target_memory_id, relationship_type, strength 
                FROM memory_relationships
            """
            df_relationships = pd.read_sql_query(query, conn)
        except pd.errors.DatabaseError:
            print("Note: 'memory_relationships' table not found. Graph analysis will be skipped.", file=sys.stderr)
            df_relationships = pd.DataFrame() # Empty DataFrame if table doesn't exist
            
        conn.close()
        return df_memories, df_relationships
    except sqlite3.Error as e:
        print(f"Database connection error: {e}", file=sys.stderr)
        return None, None

def preprocess_memories(df_m):
    """Applies necessary transformations to the memories DataFrame."""
    if df_m.empty:
        return df_m
        
    # Ensure 'created_at' is a datetime object, coercing errors
    df_m['created_at'] = pd.to_datetime(df_m['created_at'], errors='coerce')
    
    # Calculate content length, ensuring content is treated as string
    df_m['content'] = df_m['content'].astype(str)
    df_m['content_length'] = df_m['content'].str.len()
    
    # Ensure importance is numeric, fill NaNs with 0
    df_m['importance'] = pd.to_numeric(df_m['importance'], errors='coerce').fillna(0)

    # Parse tags
    df_m['parsed_tags'] = df_m['tags'].apply(safe_parse_tags)
    
    return df_m

# --- Analysis Functions ---

def analyze_temporal(df_m):
    """Analyzes memory creation frequency over time."""
    # Filter out rows where date parsing failed (NaT - Not a Time)
    df_time = df_m.dropna(subset=['created_at'])
    if df_time.empty:
        return None
        
    # Resample by month ('M') and count memories
    temporal_counts = df_time.resample('M', on='created_at').size()
    # Format the index (the month) for better readability (Year-Month)
    temporal_counts.index = temporal_counts.index.strftime('%Y-%m')
    return temporal_counts

def analyze_taxonomy(df_m):
    """Analyzes tag usage, frequency, and namespaces."""
    all_tags = []
    for tags_list in df_m['parsed_tags']:
        all_tags.extend(tags_list)

    tag_counts = Counter(all_tags)
    total_tags = len(all_tags)
    unique_tags = len(tag_counts)

    # Analyze namespaces (e.g., type:, spec:)
    namespaces = Counter()
    for tag in all_tags:
        if ':' in tag:
            # Split only on the first colon
            namespace = tag.split(':', 1)[0]
            namespaces[namespace] += 1
        else:
            namespaces['(no_namespace)'] += 1

    return tag_counts, total_tags, unique_tags, namespaces

def analyze_content_structure(df_m):
    """Analyzes adherence to the templates defined in the documentation."""
    # Defined templates in LOCAL-MEMORY-ENVIRONMENT.md Section 6

    # 1. Check for starting templates
    templates = ['[PATTERN]', '[DECISION]', '[PROBLEM]']
    # Add practical extensions often found in logs
    extended_templates = templates + ['[INSIGHT]', '[LESSON]', '[BUG]', '[FIX]']
    
    structure_counts = Counter()
    for content in df_m['content']:
        if not content or not content.strip():
            structure_counts['(empty/null)'] += 1
            continue
            
        content = content.strip()
        found = False
        for template in extended_templates:
            if content.startswith(template):
                structure_counts[template] += 1
                found = True
                break
        if not found:
            structure_counts['(unstructured)'] += 1

    # 2. Check for internal keywords (measures how often the full template structure is used)
    keywords = ['CONTEXT:', 'EVIDENCE:', 'REASONING:', 'OUTCOME:']
    keyword_counts = Counter()
    for keyword in keywords:
        # Use regex=False for literal string matching
        count = df_m['content'].str.contains(keyword, regex=False, na=False).sum()
        keyword_counts[keyword] = count

    return structure_counts, keyword_counts

def analyze_graph(df_m, df_r):
    """Analyzes the connectivity and health of the memory graph."""
    stats = {}
    total_nodes = len(df_m)
    
    if df_r.empty or 'source_memory_id' not in df_r.columns:
        stats['total_edges'] = 0
        stats['density'] = 0
        stats['orphans_count'] = total_nodes
        stats['orphans_ratio'] = 100.0 if total_nodes > 0 else 0
        stats['relationship_types'] = pd.Series(dtype='int64')
        return stats

    total_edges = len(df_r)
    stats['total_edges'] = total_edges
    
    # Density (E/N)
    stats['density'] = (total_edges / total_nodes) if total_nodes > 0 else 0
    
    # Orphans (Nodes with no relationships)
    # Find all IDs present in the relationship table
    related_ids = set(df_r['source_memory_id']).union(set(df_r['target_memory_id']))
    # Find all IDs in the memories table
    all_ids = set(df_m['id'])
    # Orphans are memories that never appear in the relationship table
    orphan_ids = all_ids - related_ids
    stats['orphans_count'] = len(orphan_ids)
    stats['orphans_ratio'] = (stats['orphans_count'] / total_nodes) * 100 if total_nodes > 0 else 0

    # Relationship Types
    stats['relationship_types'] = df_r['relationship_type'].value_counts()
    
    return stats

# --- Reporting Function ---

def print_report(stats, analysis):
    """Formats and prints the comprehensive analysis report."""
    print("\n" + "="*65)
    print(f"          Local Memory Database Comprehensive Analysis")
    print(f"          Generated: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("="*65)

    # 1. Overview
    print("\n--- 1. Overview ---")
    print(f"{'Total Memories (Nodes):':<25} {stats['total_memories']}")
    print(f"{'Total Relationships (Edges):':<25} {analysis['graph']['total_edges']}")
    print(f"{'Date Range:':<25} {stats['start_date']} to {stats['end_date']}")
    print(f"{'Avg. Memory Length:':<25} {stats['avg_length']:.0f} characters")

    # 2. Importance Distribution
    print("\n--- 2. Importance Distribution ---")
    print("Score | Count")
    print("------|------")
    # Ensure importance levels are sorted numerically descending
    importance_dist = analysis['importance'].sort_index(ascending=False)
    for score, count in importance_dist.items():
        # Display score as integer
        label = int(score)
        print(f"{label:<5} | {count}")

    # 3. Taxonomy (Domains and Tags)
    print("\n--- 3. Taxonomy Analysis ---")
    
    # 3.1 Domains
    print("\n3.1. Top 15 Domains:")
    print("Domain                     | Count")
    print("---------------------------|------")
    top_domains = analysis['domains'].nlargest(15)
    for domain, count in top_domains.items():
        # Handle potential None values gracefully
        domain_label = str(domain) if domain is not None else "(none)"
        print(f"{textwrap.shorten(domain_label, width=25, placeholder='...'):<25} | {count}")

    # 3.2 Tags
    print("\n3.2. Tag Usage:")
    print(f"{'Total Tags Used:':<25} {stats['total_tags']}")
    print(f"{'Unique Tags:':<25} {stats['unique_tags']}")
    print(f"{'Avg. Tags per Memory:':<25} {stats['avg_tags_per_memory']:.2f}")

    print("\n3.3. Tag Namespaces (Adherence to conventions):")
    print("Namespace                  | Count")
    print("---------------------------|------")
    for ns, count in analysis['namespaces'].most_common():
        print(f"{textwrap.shorten(str(ns), width=25, placeholder='...'):<25} | {count}")

    print("\n3.4. Top 20 Tags:")
    print("Tag                        | Count")
    print("---------------------------|------")
    for tag, count in analysis['tag_counts'].most_common(20):
         print(f"{textwrap.shorten(str(tag), width=25, placeholder='...'):<25} | {count}")

    # 4. Content Structure
    print("\n--- 4. Content Structure (Template Adherence) ---")
    
    print("\n4.1. Starting Template (e.g., [PATTERN]):")
    print("Template                   | Count")
    print("---------------------------|------")
    for structure, count in analysis['content_structure'].most_common():
        print(f"{structure:<25} | {count}")

    print("\n4.2. Internal Keywords (e.g., CONTEXT:):")
    print("Keyword                    | Count | Percentage")
    print("---------------------------|-------|-----------")
    total_memories = stats['total_memories']
    for keyword, count in analysis['content_keywords'].items():
        percentage = (count / total_memories) * 100 if total_memories > 0 else 0
        print(f"{keyword:<25} | {count:<5} | {percentage:.1f}%")

    # 5. Agent Contribution
    print("\n--- 5. Agent Contribution ---")
    print("Agent Type                 | Count")
    print("---------------------------|------")
    top_agents = analysis['agents'].nlargest(15)
    for agent, count in top_agents.items():
         # Handle potential None values gracefully
         agent_label = str(agent) if agent is not None else "(unknown)"
         print(f"{textwrap.shorten(agent_label, width=25, placeholder='...'):<25} | {count}")

    # 6. Graph Health & Connectivity
    print("\n--- 6. Graph Health & Connectivity ---")
    graph_stats = analysis['graph']
    print(f"{'Graph Density (E/N):':<25} {graph_stats['density']:.4f}")
    print(f"{'Orphan Memories:':<25} {graph_stats['orphans_count']} ({graph_stats['orphans_ratio']:.1f}%)")
        
    print("\n6.1. Relationship Type Distribution:")
    print("Relationship Type          | Count")
    print("---------------------------|------")
    if not graph_stats['relationship_types'].empty:
        for r_type, count in graph_stats['relationship_types'].items():
             print(f"{textwrap.shorten(str(r_type), width=25, placeholder='...'):<25} | {count}")
    else:
        print("No relationships found.")

    # 7. Temporal Analysis
    print("\n--- 7. Temporal Analysis (Memories per Month) ---")
    if analysis['temporal'] is not None and not analysis['temporal'].empty:
        # Use to_string for cleaner Series output
        print(analysis['temporal'].to_string())
    else:
        print("Could not perform temporal analysis (no valid timestamps).")

    print("\n" + "="*65)


# --- Main Execution ---

if __name__ == "__main__":
    db_path = get_db_path()

    if not db_path:
        print("\nERROR: Database file not found.", file=sys.stderr)
        print("Searched default locations and LM_DB_PATH environment variable.", file=sys.stderr)
        sys.exit(1)

    df_m, df_r = load_data(db_path)

    if df_m is not None:
        df_m = preprocess_memories(df_m)
        
        if df_m.empty:
            print("Database loaded successfully, but the 'memories' table is empty.")
            sys.exit(0)

        # Perform Analysis
        tag_counts, total_tags, unique_tags, namespaces = analyze_taxonomy(df_m)
        content_structure, content_keywords = analyze_content_structure(df_m)
        temporal_data = analyze_temporal(df_m)
        graph_stats = analyze_graph(df_m, df_r)

        analysis = {
            'importance': df_m['importance'].value_counts(),
            'domains': df_m['domain'].value_counts(),
            'agents': df_m['agent_type'].value_counts(),
            'tag_counts': tag_counts,
            'namespaces': namespaces,
            'content_structure': content_structure,
            'content_keywords': content_keywords,
            'temporal': temporal_data,
            'graph': graph_stats,
        }

        # Calculate Statistics
        min_date = df_m['created_at'].min()
        max_date = df_m['created_at'].max()
        total_memories = len(df_m)

        stats = {
            'total_memories': total_memories,
            'start_date': min_date.strftime('%Y-%m-%d') if pd.notna(min_date) else 'N/A',
            'end_date': max_date.strftime('%Y-%m-%d') if pd.notna(max_date) else 'N/A',
            'avg_length': df_m['content_length'].mean() if not df_m['content_length'].isnull().all() else 0,
            'total_tags': total_tags,
            'unique_tags': unique_tags,
            'avg_tags_per_memory': total_tags / total_memories if total_memories > 0 else 0,
        }

        print_report(stats, analysis)

    else:
        print("\nAnalysis could not be completed due to data loading errors.")
