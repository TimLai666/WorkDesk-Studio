#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def list_markdown_files(root: Path) -> set[Path]:
    if not root.exists():
        return set()
    return {p.relative_to(root) for p in root.rglob("*.md")}


def get_changed_files(base_ref: str, head_ref: str) -> set[Path]:
    diff_range = f"{base_ref}...{head_ref}"
    cmd = ["git", "diff", "--name-only", diff_range]
    try:
        out = subprocess.check_output(cmd, text=True).strip()
    except subprocess.CalledProcessError:
        fallback_cmd = ["git", "diff", "--name-only", "HEAD~1...HEAD"]
        out = subprocess.check_output(fallback_cmd, text=True).strip()
    if not out:
        return set()
    return {Path(line.strip()) for line in out.splitlines() if line.strip()}


def check_docs_sync(repo_root: Path, base_ref: str, head_ref: str) -> int:
    en_root = repo_root / "docs" / "en"
    zh_root = repo_root / "docs" / "zh-TW"

    en_files = list_markdown_files(en_root)
    zh_files = list_markdown_files(zh_root)

    missing_pairs = sorted([rel for rel in en_files if rel not in zh_files])
    if missing_pairs:
        print("Missing Traditional Chinese docs for:")
        for rel in missing_pairs:
            print(f"  - docs/en/{rel} -> docs/zh-TW/{rel}")
        return 1

    changed = get_changed_files(base_ref, head_ref)
    changed_en = sorted(
        path for path in changed if path.as_posix().startswith("docs/en/") and path.suffix == ".md"
    )
    changed_set = {path.as_posix() for path in changed}

    unsynced = []
    for en_path in changed_en:
        rel = en_path.relative_to(Path("docs/en"))
        zh_path = Path("docs/zh-TW") / rel
        if zh_path.as_posix() not in changed_set:
            unsynced.append((en_path, zh_path))

    if unsynced:
        print("The following English doc changes are missing zh-TW updates in the same PR:")
        for en_path, zh_path in unsynced:
            print(f"  - {en_path.as_posix()} requires {zh_path.as_posix()}")
        return 1

    print("Docs sync check passed.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="HEAD~1")
    parser.add_argument("--head-ref", default="HEAD")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    return check_docs_sync(repo_root, args.base_ref, args.head_ref)


if __name__ == "__main__":
    sys.exit(main())
