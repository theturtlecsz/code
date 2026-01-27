#!/usr/bin/env bash
# Documentation Archive Pack Tool
# Creates, lists, extracts, and verifies documentation archive packs

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ARCHIVE_DIR="$REPO_ROOT/archive"

usage() {
    cat << EOF
Usage: $(basename "$0") <command> [options]

Commands:
  create <source-dir>           Create archive pack from source directory
  list <pack-file>              List contents of archive pack
  extract <pack-file> [dest]    Extract archive pack to destination
  extract-file <pack> <path>    Extract single file from pack
  verify <pack-file>            Verify pack integrity against manifest
  manifest <source-dir>         Generate manifest.json without packing

Options:
  --name <name>                 Custom pack name (default: docs-pack-YYYYMMDD)
  --description <desc>          Pack description
  --format <tar.zst|zip>        Pack format (default: tar.zst)
  --include <md|all>            Include filter (default: md)

Examples:
  $(basename "$0") create docs/archive/specs
  $(basename "$0") list archive/docs-pack-20260121.tar.zst
  $(basename "$0") list archive/tree-pack-20260127-docs-archive.zip
  $(basename "$0") verify archive/docs-pack-20260121.tar.zst
EOF
}

# Generate manifest for a directory using Python for speed
generate_manifest() {
    local source_dir="$1"
    local pack_name="${2:-docs-pack-$(date +%Y%m%d)}"
    local description="${3:-Archive of $source_dir generated $(date -Iseconds)}"
    local include_mode="${4:-md}"
    local source_commit
    source_commit=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")

    python3 << PYEOF
import os
import json
import hashlib
from pathlib import Path

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            h.update(chunk)
    return h.hexdigest()

def count_lines(path):
    try:
        with open(path, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(1 for _ in f)
    except:
        return 0

repo_root = "$REPO_ROOT"
source_dir = "$source_dir"
pack_name = "$pack_name"
description = """$description"""
source_commit = "$source_commit"
include_mode = "$include_mode"

full_source = os.path.join(repo_root, source_dir)
files = []
total_lines = 0
total_bytes = 0

for root, dirs, filenames in os.walk(full_source):
    for filename in filenames:
        if include_mode == "md" and not filename.endswith('.md'):
            continue

        filepath = os.path.join(root, filename)
        rel_path = os.path.relpath(filepath, repo_root)

        lines = count_lines(filepath)
        bytes_size = os.path.getsize(filepath)
        sha256 = sha256_file(filepath)

        # Determine duplicate
        duplicate_of = None
        if '/archive/specs/' in rel_path:
            active_path = rel_path.replace('/archive/specs/', '/')
            active_full = os.path.join(repo_root, active_path)
            if os.path.exists(active_full):
                duplicate_of = active_path

        category = 'archive-tree' if include_mode == 'all' else 'archive-candidate'
        tags = ['archive']
        if 'spec' in rel_path.lower():
            tags.append('spec')
        if duplicate_of:
            tags.append('duplicate')

        files.append({
            'path': rel_path,
            'sha256': sha256,
            'lines': lines,
            'bytes': bytes_size,
            'category': category,
            'tags': tags,
            'duplicate_of': duplicate_of,
            'destination': 'archive-only'
        })

        total_lines += lines
        total_bytes += bytes_size

manifest = {
    'version': '1.0',
    'created': '$(date -Iseconds)',
    'pack_name': pack_name,
    'description': description,
    'source_commit': source_commit,
    'include_mode': include_mode,
    'stats': {
        'total_files': len(files),
        'total_lines': total_lines,
        'total_bytes': total_bytes
    },
    'files': sorted(files, key=lambda x: x['path'])
}

print(json.dumps(manifest, indent=2))
PYEOF
}

# Create archive pack
cmd_create() {
    local source_dir="$1"
    local include_mode="${INCLUDE_MODE:-md}"
    local format="${FORMAT:-tar.zst}"
    local default_pack_name="docs-pack-$(date +%Y%m%d)"
    if [[ "$include_mode" == "all" ]]; then
        local source_slug="${source_dir#./}"
        source_slug="${source_slug%/}"
        source_slug="${source_slug//\//-}"
        default_pack_name="tree-pack-$(date +%Y%m%d)-${source_slug}"
    fi
    local pack_name="${PACK_NAME:-$default_pack_name}"
    local description="${DESCRIPTION:-Archive of $source_dir}"

    if [[ ! -d "$REPO_ROOT/$source_dir" ]]; then
        echo "Error: Source directory does not exist: $source_dir" >&2
        exit 1
    fi

    if [[ "$format" != "tar.zst" && "$format" != "zip" ]]; then
        echo "Error: Unsupported format: $format (expected tar.zst or zip)" >&2
        exit 1
    fi
    if [[ "$include_mode" != "md" && "$include_mode" != "all" ]]; then
        echo "Error: Unsupported include mode: $include_mode (expected md or all)" >&2
        exit 1
    fi

    local create_staging_dir
    create_staging_dir=$(mktemp -d)

    echo "Creating archive pack: $pack_name"
    echo "Source: $source_dir"
    echo "Format: $format"
    echo "Include: $include_mode"

    # Create directory structure
    mkdir -p "$create_staging_dir/files"

    # Generate manifest
    echo "Generating manifest..."
    generate_manifest "$source_dir" "$pack_name" "$description" "$include_mode" > "$create_staging_dir/manifest.json"

    # Copy files preserving structure
    echo "Copying files..."
    local file_count=0
    local find_args=()
    if [[ "$include_mode" == "md" ]]; then
        find_args=(-name "*.md")
    fi
    while IFS= read -r -d '' file || [[ -n "$file" ]]; do
        local rel_path="${file#$REPO_ROOT/}"
        local dest_dir="$create_staging_dir/files/$(dirname "$rel_path")"
        mkdir -p "$dest_dir"
        cp "$file" "$dest_dir/"
        file_count=$((file_count + 1))
    done < <(find "$REPO_ROOT/$source_dir" "${find_args[@]}" -type f -print0)

    echo "Packed $file_count files"

    # Create compressed archive
    mkdir -p "$ARCHIVE_DIR"

    local pack_file="$ARCHIVE_DIR/${pack_name}.${format}"
    echo "Writing to $pack_file..."
    if [[ "$format" == "zip" ]]; then
        (cd "$create_staging_dir" && zip -qr "$pack_file" .)
    else
        tar -C "$create_staging_dir" -cf - . | zstd -19 -T0 > "$pack_file"
    fi

    local pack_size
    pack_size=$(du -h "$pack_file" | cut -f1)
    echo "Created: $pack_file ($pack_size)"

    # Clean up staging
    rm -rf "$create_staging_dir"

    # Verify
    echo "Verifying..."
    if cmd_verify "$pack_file" >/dev/null 2>&1; then
        echo "Verification: PASSED"
    else
        echo "Verification: FAILED" >&2
        exit 1
    fi
}

read_manifest() {
    local pack_file="$1"
    if [[ "$pack_file" == *.zip ]]; then
        unzip -p "$pack_file" manifest.json
    else
        zstd -d -c "$pack_file" | tar -xOf - manifest.json
    fi
}

extract_pack() {
    local pack_file="$1"
    local dest_dir="$2"
    if [[ "$pack_file" == *.zip ]]; then
        unzip -q "$pack_file" -d "$dest_dir"
    else
        zstd -d -c "$pack_file" | tar -xf - -C "$dest_dir"
    fi
}

extract_file_from_pack() {
    local pack_file="$1"
    local file_path="$2"
    if [[ "$pack_file" == *.zip ]]; then
        unzip -p "$pack_file" "$file_path"
    else
        zstd -d -c "$pack_file" | tar -xOf - "$file_path"
    fi
}

# List pack contents
cmd_list() {
    local pack_file="$1"

    if [[ ! -f "$pack_file" ]]; then
        echo "Error: Pack file not found: $pack_file" >&2
        exit 1
    fi

    echo "Pack: $pack_file"
    echo ""

    # Extract and display manifest
    local manifest
    manifest=$(read_manifest "$pack_file")

    if echo "$manifest" | jq -e 'type == "object"' >/dev/null 2>&1; then
        echo "=== Manifest ==="
        echo "$manifest" | jq '{
            version,
            created,
            pack_name,
            description,
            source_commit,
            include_mode,
            stats
        }'

        echo ""
        echo "=== Files (top 20 by size) ==="
        echo "$manifest" | jq -r '.files | sort_by(-.bytes) | .[0:20][] | "\(.bytes)\t\(.lines)\t\(.path)"' | \
            column -t -s $'\t' -N "BYTES,LINES,PATH" 2>/dev/null || \
            echo "$manifest" | jq -r '.files | sort_by(-.bytes) | .[0:20][] | "\(.bytes)\t\(.lines)\t\(.path)"'

        echo ""
        local total
        total=$(echo "$manifest" | jq '.stats.total_files')
        echo "Total: $total files"
    else
        echo "=== Manifest (raw) ==="
        echo "$manifest"
    fi
}

# Extract pack
cmd_extract() {
    local pack_file="$1"
    local dest_dir="${2:-.}"

    if [[ ! -f "$pack_file" ]]; then
        echo "Error: Pack file not found: $pack_file" >&2
        exit 1
    fi

    echo "Extracting $pack_file to $dest_dir..."
    mkdir -p "$dest_dir"
    extract_pack "$pack_file" "$dest_dir"

    echo "Extracted to: $dest_dir"
    echo "Files are in: $dest_dir/files/"
}

# Extract single file
cmd_extract_file() {
    local pack_file="$1"
    local file_path="$2"

    if [[ ! -f "$pack_file" ]]; then
        echo "Error: Pack file not found: $pack_file" >&2
        exit 1
    fi

    extract_file_from_pack "$pack_file" "files/$file_path"
}

# Verify pack integrity
cmd_verify() {
    local pack_file="$1"
    local verbose="${2:-false}"

    if [[ ! -f "$pack_file" ]]; then
        echo "Error: Pack file not found: $pack_file" >&2
        exit 1
    fi

    local verify_staging_dir
    verify_staging_dir=$(mktemp -d)

    # Extract
    extract_pack "$pack_file" "$verify_staging_dir"

    # Validate manifest
    if [[ ! -f "$verify_staging_dir/manifest.json" ]]; then
        echo "FAIL: manifest.json missing" >&2
        rm -rf "$verify_staging_dir"
        return 1
    fi

    # Use Python for fast verification
    local result
    result=$(python3 << PYEOF
import os
import json
import hashlib
import sys

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            h.update(chunk)
    return h.hexdigest()

staging_dir = "$verify_staging_dir"
verbose = "$verbose" == "true"

with open(os.path.join(staging_dir, 'manifest.json')) as f:
    manifest = json.load(f)

errors = 0
for entry in manifest['files']:
    path = entry['path']
    expected_sha256 = entry['sha256']
    file_path = os.path.join(staging_dir, 'files', path)

    if not os.path.exists(file_path):
        print(f"FAIL: Missing file: {path}", file=sys.stderr)
        errors += 1
        continue

    actual_sha256 = sha256_file(file_path)
    if expected_sha256 != actual_sha256:
        print(f"FAIL: Checksum mismatch: {path}", file=sys.stderr)
        print(f"  Expected: {expected_sha256}", file=sys.stderr)
        print(f"  Actual:   {actual_sha256}", file=sys.stderr)
        errors += 1
    elif verbose:
        print(f"OK: {path}")

if errors == 0:
    if verbose:
        print("All files verified successfully")
    print("VERIFY_OK")
else:
    print(f"Verification failed with {errors} errors", file=sys.stderr)
    print("VERIFY_FAIL")
PYEOF
)
    # Clean up
    rm -rf "$verify_staging_dir"

    if [[ "$result" == *"VERIFY_OK"* ]]; then
        return 0
    else
        return 1
    fi
}

# Generate manifest only
cmd_manifest() {
    local source_dir="$1"
    generate_manifest "$source_dir" "${PACK_NAME:-}" "${DESCRIPTION:-}" "${INCLUDE_MODE:-md}"
}

# Main
main() {
    if [[ $# -eq 0 ]]; then
        usage
        exit 1
    fi

    local command="$1"
    shift

    # Parse global options
    while [[ $# -gt 0 ]] && [[ "$1" == --* ]]; do
        case "$1" in
            --name)
                PACK_NAME="$2"
                shift 2
                ;;
            --description)
                DESCRIPTION="$2"
                shift 2
                ;;
            --format)
                FORMAT="$2"
                shift 2
                ;;
            --include)
                INCLUDE_MODE="$2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1" >&2
                exit 1
                ;;
        esac
    done

    case "$command" in
        create)
            [[ $# -lt 1 ]] && { echo "Error: source-dir required" >&2; exit 1; }
            cmd_create "$1"
            ;;
        list)
            [[ $# -lt 1 ]] && { echo "Error: pack-file required" >&2; exit 1; }
            cmd_list "$1"
            ;;
        extract)
            [[ $# -lt 1 ]] && { echo "Error: pack-file required" >&2; exit 1; }
            cmd_extract "$1" "${2:-}"
            ;;
        extract-file)
            [[ $# -lt 2 ]] && { echo "Error: pack-file and path required" >&2; exit 1; }
            cmd_extract_file "$1" "$2"
            ;;
        verify)
            [[ $# -lt 1 ]] && { echo "Error: pack-file required" >&2; exit 1; }
            cmd_verify "$1" "true"
            ;;
        manifest)
            [[ $# -lt 1 ]] && { echo "Error: source-dir required" >&2; exit 1; }
            cmd_manifest "$1"
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            echo "Unknown command: $command" >&2
            usage
            exit 1
            ;;
    esac
}

main "$@"
