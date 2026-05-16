#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "duckdb==1.5.2",
# ]
# ///
from __future__ import annotations

import argparse
import hashlib
import importlib
import os
import re
import subprocess
import sys
import time
import tomllib
import uuid
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlsplit

duckdb: Any = importlib.import_module("duckdb")

SCHEMA_VERSION = 2
GENERATOR_VERSION = "0.3.0"
COMMIT_LIMIT = 250
TERM_LIMIT = 500
CONTENT_PREFIXES = ("docs/", "website/content/", "website/static/")
CONTENT_EXCLUDES = (
    "website/static/data/meta.ddb",
    "website/static/data/meta.ddb.wal",
    "website/static/vendor/plotly/plotly-3.4.0.min.js",
)
STOP_WORDS = {
    "about",
    "after",
    "again",
    "all",
    "also",
    "and",
    "are",
    "because",
    "before",
    "being",
    "between",
    "build",
    "built",
    "can",
    "code",
    "config",
    "content",
    "could",
    "data",
    "default",
    "does",
    "each",
    "false",
    "file",
    "files",
    "for",
    "from",
    "had",
    "has",
    "have",
    "how",
    "into",
    "like",
    "more",
    "must",
    "name",
    "new",
    "one",
    "only",
    "other",
    "page",
    "pages",
    "path",
    "pre",
    "root",
    "section",
    "sections",
    "site",
    "static",
    "string",
    "than",
    "that",
    "the",
    "their",
    "them",
    "then",
    "there",
    "these",
    "this",
    "through",
    "true",
    "two",
    "url",
    "use",
    "used",
    "using",
    "value",
    "was",
    "were",
    "will",
    "when",
    "where",
    "which",
    "with",
    "without",
    "would",
    "you",
    "your",
    "zorto",
}
LINK_RE = re.compile(
    r"\[[^\]]+\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)|href=[\"']([^\"']+)[\"']",
    re.IGNORECASE,
)


@dataclass
class BuildRun:
    run_id: str
    started_at: str
    finished_at: str
    duration_ms: int
    status: str
    zorto_version: str
    command: str


@dataclass
class Commit:
    sha: str
    short_sha: str
    committed_at: str
    author_name: str
    subject: str
    file_count: int = 0
    additions: int = 0
    deletions: int = 0


@dataclass
class ContentFile:
    path: str
    kind: str
    title: str | None
    bytes: int
    word_count: int
    last_commit_sha: str | None
    last_commit_at: str | None
    text: str


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate zorto.dev metadata database")
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--website-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--build-output", type=Path)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    website_dir = args.website_dir.resolve()
    output = args.output or website_dir / "static" / "data" / "meta.ddb"
    build_output = args.build_output or repo_root / "target" / "zorto-meta-public"

    previous_runs = read_previous_build_runs(output)
    build_run = run_zorto_build(repo_root, website_dir, build_output)
    build_outputs = collect_build_outputs(build_output)

    tmp = output.with_name(f".{output.name}.{uuid.uuid4().hex}.tmp")
    cleanup_duckdb_files(tmp)
    output.parent.mkdir(parents=True, exist_ok=True)

    con = duckdb.connect(str(tmp))
    try:
        write_database(con, repo_root, previous_runs + [build_run], build_outputs)
        con.close()
        cleanup_duckdb_files(output)
        os.replace(tmp, output)
    finally:
        try:
            con.close()
        except Exception:
            pass
        cleanup_duckdb_files(tmp)

    print(f"wrote {rel(repo_root, output)}")
    return 0


def write_database(
    con: Any,
    repo_root: Path,
    build_runs: list[BuildRun],
    build_outputs: list[tuple[str, int, str, str, str]],
) -> None:
    create_schema(con)
    insert_meta_info(con)
    insert_repo_snapshot(con, repo_root)
    commits = collect_commits(repo_root)
    insert_commits(con, commits)
    insert_commit_daily(con, commits)
    insert_packages(con, repo_root)
    content_files = collect_content_files(repo_root)
    insert_content_files(con, content_files)
    insert_content_terms(con, content_files)
    insert_content_links(con, repo_root, content_files, build_outputs)
    insert_build_runs(con, build_runs)
    con.executemany("INSERT INTO build_outputs VALUES (?, ?, ?, ?, ?)", build_outputs)
    con.execute("CHECKPOINT")


def create_schema(con: Any) -> None:
    con.execute(
        """
        CREATE TABLE meta_info (
            schema_version INTEGER NOT NULL,
            generated_at TIMESTAMP NOT NULL,
            generator_version VARCHAR NOT NULL,
            duckdb_version VARCHAR NOT NULL
        );
        CREATE TABLE repo_snapshot (
            branch VARCHAR NOT NULL,
            head_sha VARCHAR NOT NULL,
            dirty BOOLEAN NOT NULL,
            tracked_count INTEGER NOT NULL,
            untracked_count INTEGER NOT NULL
        );
        CREATE TABLE commits (
            sha VARCHAR NOT NULL,
            short_sha VARCHAR NOT NULL,
            committed_at TIMESTAMP NOT NULL,
            author_name VARCHAR NOT NULL,
            subject VARCHAR NOT NULL,
            file_count INTEGER NOT NULL,
            additions INTEGER NOT NULL,
            deletions INTEGER NOT NULL
        );
        CREATE TABLE commit_daily (
            day DATE NOT NULL,
            commit_count INTEGER NOT NULL,
            file_count INTEGER NOT NULL,
            additions INTEGER NOT NULL,
            deletions INTEGER NOT NULL
        );
        CREATE TABLE packages (
            ecosystem VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            version VARCHAR NOT NULL,
            manifest_path VARCHAR NOT NULL
        );
        CREATE TABLE content_files (
            path VARCHAR NOT NULL,
            kind VARCHAR NOT NULL,
            title VARCHAR,
            bytes INTEGER NOT NULL,
            word_count INTEGER NOT NULL,
            last_commit_sha VARCHAR,
            last_commit_at TIMESTAMP
        );
        CREATE TABLE content_terms (
            term VARCHAR NOT NULL,
            file_count INTEGER NOT NULL,
            occurrence_count INTEGER NOT NULL
        );
        CREATE TABLE content_links (
            source_path VARCHAR NOT NULL,
            target VARCHAR NOT NULL,
            target_path VARCHAR,
            link_kind VARCHAR NOT NULL,
            target_exists BOOLEAN
        );
        CREATE TABLE build_runs (
            run_id VARCHAR NOT NULL,
            started_at TIMESTAMP NOT NULL,
            finished_at TIMESTAMP NOT NULL,
            duration_ms INTEGER NOT NULL,
            status VARCHAR NOT NULL,
            zorto_version VARCHAR NOT NULL,
            command VARCHAR NOT NULL
        );
        CREATE TABLE build_outputs (
            path VARCHAR NOT NULL,
            bytes INTEGER NOT NULL,
            extension VARCHAR NOT NULL,
            kind VARCHAR NOT NULL,
            sha256 VARCHAR NOT NULL
        );
        """
    )


def insert_meta_info(con: Any) -> None:
    con.execute(
        "INSERT INTO meta_info VALUES (?, ?, ?, ?)",
        [SCHEMA_VERSION, now_iso(), GENERATOR_VERSION, duckdb.__version__],
    )


def insert_repo_snapshot(con: Any, repo_root: Path) -> None:
    tracked = git(repo_root, "ls-files").splitlines()
    status_lines = git(repo_root, "status", "--porcelain=v1").splitlines()
    untracked_count = sum(1 for line in status_lines if line.startswith("?? "))
    branch = git(repo_root, "branch", "--show-current").strip() or "(detached)"
    head_sha = git(repo_root, "rev-parse", "HEAD").strip()
    con.execute(
        "INSERT INTO repo_snapshot VALUES (?, ?, ?, ?, ?)",
        [branch, head_sha, bool(status_lines), len(tracked), untracked_count],
    )


def collect_commits(repo_root: Path) -> list[Commit]:
    output = git(
        repo_root,
        "log",
        f"--max-count={COMMIT_LIMIT}",
        "--date=iso-strict",
        "--format=commit:%H%x1f%h%x1f%cI%x1f%an%x1f%s",
        "--numstat",
    )
    commits: list[Commit] = []
    current: Commit | None = None
    for line in output.splitlines():
        if line.startswith("commit:"):
            if current is not None:
                commits.append(current)
            sha, short_sha, committed_at, author_name, subject = line[
                len("commit:") :
            ].split("\x1f", 4)
            current = Commit(
                sha=sha,
                short_sha=short_sha,
                committed_at=committed_at,
                author_name=author_name,
                subject=subject,
            )
        elif current is not None and line.strip():
            parts = line.split("\t")
            if len(parts) >= 3:
                current.file_count += 1
                if parts[0].isdigit():
                    current.additions += int(parts[0])
                if parts[1].isdigit():
                    current.deletions += int(parts[1])
    if current is not None:
        commits.append(current)
    return commits


def insert_commits(con: Any, commits: list[Commit]) -> None:
    con.executemany(
        "INSERT INTO commits VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                c.sha,
                c.short_sha,
                c.committed_at,
                c.author_name,
                c.subject,
                c.file_count,
                c.additions,
                c.deletions,
            )
            for c in commits
        ],
    )


def insert_commit_daily(con: Any, commits: list[Commit]) -> None:
    daily: dict[str, dict[str, int]] = defaultdict(
        lambda: {"commit_count": 0, "file_count": 0, "additions": 0, "deletions": 0}
    )
    for commit in commits:
        day = commit.committed_at[:10]
        daily[day]["commit_count"] += 1
        daily[day]["file_count"] += commit.file_count
        daily[day]["additions"] += commit.additions
        daily[day]["deletions"] += commit.deletions
    con.executemany(
        "INSERT INTO commit_daily VALUES (?, ?, ?, ?, ?)",
        [
            (
                day,
                vals["commit_count"],
                vals["file_count"],
                vals["additions"],
                vals["deletions"],
            )
            for day, vals in sorted(daily.items())
        ],
    )


def insert_packages(con: Any, repo_root: Path) -> None:
    rows: list[tuple[str, str, str, str]] = []
    root_cargo = read_toml(repo_root / "Cargo.toml")
    workspace_version = str(
        root_cargo.get("workspace", {}).get("package", {}).get("version", "")
    )
    for manifest in sorted(repo_root.glob("**/Cargo.toml")):
        if should_skip_path(repo_root, manifest):
            continue
        data = read_toml(manifest)
        pkg = data.get("package")
        if not pkg:
            continue
        version = pkg.get("version", "")
        if isinstance(version, dict) and version.get("workspace"):
            version = workspace_version
        rows.append(
            ("rust", str(pkg.get("name", "")), str(version), rel(repo_root, manifest))
        )

    for manifest in sorted(repo_root.glob("**/pyproject.toml")):
        if should_skip_path(repo_root, manifest):
            continue
        data = read_toml(manifest)
        project = data.get("project")
        if project:
            rows.append(
                (
                    "python",
                    str(project.get("name", "")),
                    str(project.get("version", "")),
                    rel(repo_root, manifest),
                )
            )

    con.executemany("INSERT INTO packages VALUES (?, ?, ?, ?)", rows)


def collect_content_files(repo_root: Path) -> list[ContentFile]:
    rows: list[ContentFile] = []
    for rel_path in git(repo_root, "ls-files").splitlines():
        if not rel_path.startswith(CONTENT_PREFIXES) or rel_path in CONTENT_EXCLUDES:
            continue
        path = repo_root / rel_path
        if not path.is_file():
            continue
        text = read_text_best_effort(path)
        last_sha, last_at = last_commit_for_path(repo_root, rel_path)
        rows.append(
            ContentFile(
                path=rel_path,
                kind=classify_content_file(rel_path),
                title=extract_title(text, path.suffix),
                bytes=path.stat().st_size,
                word_count=count_words(text),
                last_commit_sha=last_sha,
                last_commit_at=last_at,
                text=text,
            )
        )
    return rows


def insert_content_files(con: Any, files: list[ContentFile]) -> None:
    con.executemany(
        "INSERT INTO content_files VALUES (?, ?, ?, ?, ?, ?, ?)",
        [
            (
                f.path,
                f.kind,
                f.title,
                f.bytes,
                f.word_count,
                f.last_commit_sha,
                f.last_commit_at,
            )
            for f in files
        ],
    )


def insert_content_terms(con: Any, files: list[ContentFile]) -> None:
    counts: Counter[str] = Counter()
    files_by_term: defaultdict[str, set[str]] = defaultdict(set)
    for file in files:
        if file.kind not in {"content", "docs"} or not file.text:
            continue
        for term in extract_terms(file.text):
            counts[term] += 1
            files_by_term[term].add(file.path)
    rows = [
        (term, len(files_by_term[term]), count)
        for term, count in counts.most_common(TERM_LIMIT)
    ]
    con.executemany("INSERT INTO content_terms VALUES (?, ?, ?)", rows)


def insert_content_links(
    con: Any,
    repo_root: Path,
    files: list[ContentFile],
    build_outputs: list[tuple[str, int, str, str, str]],
) -> None:
    tracked = set(git(repo_root, "ls-files").splitlines())
    output_paths = {row[0] for row in build_outputs}
    rows = []
    for file in files:
        if file.kind not in {"content", "docs", "asset-code"} or not file.text:
            continue
        for target in extract_links(file.text):
            resolved = resolve_local_link(
                repo_root, file.path, target, tracked, output_paths
            )
            if resolved is None:
                continue
            target_path, link_kind, exists = resolved
            rows.append((file.path, target, target_path, link_kind, exists))
    con.executemany("INSERT INTO content_links VALUES (?, ?, ?, ?, ?)", rows)


def insert_build_runs(con: Any, runs: list[BuildRun]) -> None:
    con.executemany(
        "INSERT INTO build_runs VALUES (?, ?, ?, ?, ?, ?, ?)",
        [
            (
                run.run_id,
                run.started_at,
                run.finished_at,
                run.duration_ms,
                run.status,
                run.zorto_version,
                run.command,
            )
            for run in runs
        ],
    )


def run_zorto_build(repo_root: Path, website_dir: Path, build_output: Path) -> BuildRun:
    build_output.mkdir(parents=True, exist_ok=True)
    command = [
        "cargo",
        "run",
        "-p",
        "zorto",
        "--",
        "--root",
        "website",
        "--sandbox",
        ".",
        "build",
        "--output",
        str(build_output),
    ]
    public_command = "cargo run -p zorto -- --root website --sandbox . build --output target/zorto-meta-public"
    started = now_iso()
    started_monotonic = time.monotonic()
    env = os.environ.copy()
    env["VIRTUAL_ENV"] = str(website_dir / ".venv")
    subprocess.run(command, cwd=repo_root, check=True, env=env)
    finished = now_iso()
    return BuildRun(
        run_id=uuid.uuid4().hex,
        started_at=started,
        finished_at=finished,
        duration_ms=round((time.monotonic() - started_monotonic) * 1000),
        status="success",
        zorto_version=zorto_version(repo_root),
        command=public_command,
    )


def collect_build_outputs(build_output: Path) -> list[tuple[str, int, str, str, str]]:
    rows = []
    for path in sorted(build_output.rglob("*")):
        if not path.is_file():
            continue
        output_rel = path.relative_to(build_output).as_posix()
        suffix = path.suffix.lower().lstrip(".") or "(none)"
        rows.append(
            (
                output_rel,
                path.stat().st_size,
                suffix,
                classify_output(path),
                sha256_file(path),
            )
        )
    return rows


def read_previous_build_runs(output: Path) -> list[BuildRun]:
    if not output.exists():
        return []
    try:
        con = duckdb.connect(str(output), read_only=True)
        rows = con.execute(
            "SELECT run_id, started_at, finished_at, duration_ms, status, zorto_version, command FROM build_runs ORDER BY started_at"
        ).fetchall()
        con.close()
    except Exception:
        return []
    return [
        BuildRun(
            run_id=str(row[0]),
            started_at=to_iso(row[1]),
            finished_at=to_iso(row[2]),
            duration_ms=int(row[3]),
            status=str(row[4]),
            zorto_version=str(row[5]),
            command=str(row[6]),
        )
        for row in rows
    ]


def git(repo_root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo_root, text=True).rstrip(
        "\n"
    )


def read_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def should_skip_path(repo_root: Path, path: Path) -> bool:
    rel_path = rel(repo_root, path)
    return (
        rel_path.startswith("target/")
        or rel_path.startswith("external/")
        or rel_path.startswith("website/.venv/")
        or rel_path.startswith("website/public/")
    )


def read_text_best_effort(path: Path) -> str:
    if path.suffix.lower() not in {
        ".md",
        ".toml",
        ".txt",
        ".html",
        ".css",
        ".js",
        ".svg",
    }:
        return ""
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return ""


def last_commit_for_path(
    repo_root: Path, rel_path: str
) -> tuple[str | None, str | None]:
    try:
        out = git(repo_root, "log", "-1", "--format=%H%x1f%cI", "--", rel_path)
    except subprocess.CalledProcessError:
        return None, None
    if not out:
        return None, None
    sha, committed_at = out.split("\x1f", 1)
    return sha, committed_at


def extract_title(text: str, suffix: str) -> str | None:
    if not text:
        return None
    if suffix.lower() == ".md" and text.startswith("+++\n"):
        end = text.find("\n+++", 4)
        if end != -1:
            try:
                frontmatter = tomllib.loads(text[4:end])
                title = frontmatter.get("title")
                if isinstance(title, str):
                    return title
            except tomllib.TOMLDecodeError:
                pass
    for line in text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return None


def count_words(text: str) -> int:
    if not text:
        return 0
    return len(re.findall(r"[A-Za-z0-9_']+", strip_frontmatter(text)))


def extract_terms(text: str) -> list[str]:
    text = strip_code(strip_frontmatter(text))
    terms = []
    for raw in re.findall(r"[A-Za-z][A-Za-z0-9_'-]{2,}", text.lower()):
        term = raw.strip("_'-")
        if len(term) < 3 or term in STOP_WORDS or term.isdigit():
            continue
        terms.append(term)
    return terms


def extract_links(text: str) -> list[str]:
    text = strip_code(strip_frontmatter(text))
    links = []
    for match in LINK_RE.finditer(text):
        target = (match.group(1) or match.group(2) or "").strip()
        if target and is_real_local_link_candidate(target):
            links.append(target)
    return links


def resolve_local_link(
    repo_root: Path,
    source_path: str,
    target: str,
    tracked: set[str],
    output_paths: set[str],
) -> tuple[str | None, str, bool | None] | None:
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc:
        return None
    if parsed.path.startswith(("mailto:", "tel:", "javascript:")):
        return None

    link_path = unquote(parsed.path)
    if not link_path:
        return (source_path, "anchor", True)

    if link_path.startswith("/"):
        output_target = link_path.strip("/")
        candidates = site_route_candidates(output_target)
        existing = next((c for c in candidates if c in output_paths), None)
        return (existing or output_target, "site", existing is not None)

    source_dir = Path(source_path).parent
    normalized = (source_dir / link_path).as_posix()
    normalized = os.path.normpath(normalized).replace(os.sep, "/")
    if normalized.startswith("../"):
        return (normalized, "repo", (repo_root / normalized).exists())
    return (
        normalized,
        "repo",
        normalized in tracked or (repo_root / normalized).exists(),
    )


def site_route_candidates(path: str) -> list[str]:
    if not path:
        return ["index.html"]
    candidates = [path]
    if path.endswith("/"):
        candidates.append(path + "index.html")
    elif "." not in Path(path).name:
        candidates.append(path + "/index.html")
        candidates.append(path + ".html")
    return candidates


def strip_frontmatter(text: str) -> str:
    if text.startswith("+++\n"):
        end = text.find("\n+++", 4)
        if end != -1:
            return text[end + 5 :]
    return text


def strip_code(text: str) -> str:
    text = re.sub(r"```.*?```", " ", text, flags=re.DOTALL)
    text = re.sub(r"<pre\b[^>]*>.*?</pre>", " ", text, flags=re.DOTALL | re.IGNORECASE)
    text = re.sub(
        r"<code\b[^>]*>.*?</code>", " ", text, flags=re.DOTALL | re.IGNORECASE
    )
    text = re.sub(
        r"\{%\s*tree\b.*?%\}.*?\{%\s*end\s*%\}",
        " ",
        text,
        flags=re.DOTALL | re.IGNORECASE,
    )
    return re.sub(r"`[^`\n]+`", " ", text)


def is_real_local_link_candidate(target: str) -> bool:
    if any(marker in target for marker in ("{{", "{%", "}}", "%}", "&#")):
        return False
    if target.startswith("@/"):
        return False
    return bool(target)


def classify_content_file(rel_path: str) -> str:
    suffix = Path(rel_path).suffix.lower()
    if rel_path.startswith("docs/"):
        return "docs"
    if rel_path.startswith("website/content/"):
        return "content"
    if suffix in {".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico"}:
        return "image"
    if suffix in {".css", ".js"}:
        return "asset-code"
    if suffix == ".ddb":
        return "database"
    return "static"


def classify_output(path: Path) -> str:
    suffix = path.suffix.lower()
    if suffix == ".html":
        return "html"
    if suffix in {".css", ".js"}:
        return "asset-code"
    if suffix in {".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico"}:
        return "image"
    if suffix in {".db", ".ddb"}:
        return "database"
    if suffix in {".xml", ".txt", ".md"}:
        return "text"
    return "asset"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def zorto_version(repo_root: Path) -> str:
    data = read_toml(repo_root / "crates" / "zorto-cli" / "Cargo.toml")
    version = data.get("package", {}).get("version")
    if isinstance(version, dict) and version.get("workspace"):
        version = (
            read_toml(repo_root / "Cargo.toml")
            .get("workspace", {})
            .get("package", {})
            .get("version", "")
        )
    return str(version or "")


def cleanup_duckdb_files(path: Path) -> None:
    for candidate in (path, path.with_name(path.name + ".wal")):
        if candidate.exists():
            candidate.unlink()


def rel(root: Path, path: Path) -> str:
    return path.resolve().relative_to(root.resolve()).as_posix()


def now_iso() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat()


def to_iso(value: object) -> str:
    if isinstance(value, datetime):
        if value.tzinfo is None:
            value = value.replace(tzinfo=UTC)
        return value.isoformat()
    return str(value)


if __name__ == "__main__":
    sys.exit(main())
