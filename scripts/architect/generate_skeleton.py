#!/usr/bin/env python3
"""
Repository Skeleton Extraction
Extracts public API surface from Rust files as lightweight XML.
"""

import re
from pathlib import Path
from datetime import datetime

def extract_public_api(content: str) -> list[str]:
    """Extract public API declarations from Rust source."""
    patterns = [
        # Public functions
        r"^\s*(pub(?:\([^)]+\))?\s+(?:async\s+)?fn\s+\w+[^{]*)",
        # Public structs
        r"^\s*(pub(?:\([^)]+\))?\s+struct\s+\w+[^{]*)",
        # Public enums
        r"^\s*(pub(?:\([^)]+\))?\s+enum\s+\w+[^{]*)",
        # Public traits
        r"^\s*(pub(?:\([^)]+\))?\s+trait\s+\w+[^{]*)",
        # Impl blocks (both inherent and trait impls)
        r"^\s*(impl(?:<[^>]+>)?\s+(?:\w+\s+for\s+)?\w+[^{]*)",
        # Module declarations
        r"^\s*(pub(?:\([^)]+\))?\s+mod\s+\w+)",
        # Type aliases
        r"^\s*(pub(?:\([^)]+\))?\s+type\s+\w+[^;]*)",
        # Constants
        r"^\s*(pub(?:\([^)]+\))?\s+const\s+\w+[^;]*)",
    ]

    declarations = []
    for pattern in patterns:
        for match in re.finditer(pattern, content, re.MULTILINE):
            decl = match.group(1).strip()
            # Clean up multi-line declarations
            decl = re.sub(r"\s+", " ", decl)
            # Truncate long declarations
            if len(decl) > 200:
                decl = decl[:197] + "..."
            declarations.append(decl)

    return declarations

def escape_xml(text: str) -> str:
    """Escape XML special characters."""
    return (text
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace('"', "&quot;")
            .replace("'", "&apos;"))

def main():
    repo_root = Path(__file__).parent.parent
    codex_rs = repo_root / "codex-rs"

    # Find all Rust files in core/ and tui/
    target_dirs = [
        codex_rs / "core",
        codex_rs / "tui",
    ]

    xml_parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<!-- Repository Skeleton - Generated {datetime.now().isoformat()} -->',
        '<repository name="codex-rs">',
    ]

    total_files = 0
    total_declarations = 0

    for target_dir in target_dirs:
        if not target_dir.exists():
            continue

        module_name = target_dir.name
        xml_parts.append(f'  <module name="{module_name}">')

        rust_files = sorted(target_dir.glob("**/*.rs"))

        for path in rust_files:
            try:
                content = path.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            declarations = extract_public_api(content)
            if not declarations:
                continue

            total_files += 1
            total_declarations += len(declarations)

            rel_path = path.relative_to(codex_rs)
            xml_parts.append(f'    <file path="{rel_path}">')

            for decl in declarations:
                escaped = escape_xml(decl)
                xml_parts.append(f'      <decl>{escaped}</decl>')

            xml_parts.append('    </file>')

        xml_parts.append('  </module>')

    xml_parts.append('</repository>')

    # Add summary comment
    summary = f'<!-- Summary: {total_files} files, {total_declarations} declarations -->'
    xml_parts.insert(2, summary)

    output_path = Path(__file__).parent / "repo_skeleton.xml"
    output_path.write_text("\n".join(xml_parts))

    print(f"Extracted API from {total_files} files")
    print(f"Total declarations: {total_declarations}")
    print(f"Written to {output_path}")

if __name__ == "__main__":
    main()
