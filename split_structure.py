#!/usr/bin/env python3
"""
split_structure.py - Split large repo_structure.md into uploadable parts

Splits the file into N parts of roughly equal size without breaking lines.
"""

import sys
from pathlib import Path


def split_file(input_path: Path, num_parts: int = 4) -> list[Path]:
    """Split file into roughly equal parts without breaking lines."""

    content = input_path.read_text()
    lines = content.splitlines(keepends=True)

    total_size = len(content)
    target_size = total_size // num_parts

    print(f"📄 Input: {input_path.name}")
    print(f"📊 Total size: {total_size / (1024*1024):.2f} MB")
    print(f"🎯 Target per part: ~{target_size / (1024*1024):.2f} MB")
    print()

    parts: list[list[str]] = []
    current_part: list[str] = []
    current_size = 0

    for line in lines:
        current_part.append(line)
        current_size += len(line)

        # Check if we've reached target size and should start new part
        # (but not if we're already on the last part)
        if current_size >= target_size and len(parts) < num_parts - 1:
            parts.append(current_part)
            current_part = []
            current_size = 0

    # Add remaining content as final part
    if current_part:
        parts.append(current_part)

    # Write parts to files
    output_dir = input_path.parent
    output_files: list[Path] = []

    for i, part_lines in enumerate(parts, 1):
        part_content = "".join(part_lines)

        # Add header to each part
        header = f"# Repository Structure - Part {i} of {len(parts)}\n\n"
        if i > 1:
            header += "_Continued from previous part..._\n\n"

        # Add footer
        footer = ""
        if i < len(parts):
            footer = f"\n\n_Continued in Part {i+1}..._\n"

        output_path = output_dir / f"structure_part_{i}.md"
        output_path.write_text(header + part_content + footer)
        output_files.append(output_path)

        size_mb = len(header + part_content + footer) / (1024 * 1024)
        print(f"✓ {output_path.name}: {size_mb:.2f} MB ({len(part_lines):,} lines)")

    return output_files


def main():
    context_dir = Path("notebooklm_context_diet")
    input_file = context_dir / "repo_structure.md"

    if not input_file.exists():
        print(f"❌ {input_file} not found")
        return 1

    print("=" * 60)
    print("📂 Splitting repo_structure.md for NotebookLM")
    print("=" * 60)
    print()

    output_files = split_file(input_file, num_parts=4)

    print()
    print("=" * 60)
    print("📋 Upload Order for NotebookLM:")
    print("=" * 60)
    print()

    for i, f in enumerate(output_files, 1):
        size_mb = f.stat().st_size / (1024 * 1024)
        print(f"  {i}. {f.name} ({size_mb:.2f} MB)")

    print()
    print(f"📂 Files saved in: {context_dir.absolute()}/")

    return 0


if __name__ == "__main__":
    sys.exit(main())
