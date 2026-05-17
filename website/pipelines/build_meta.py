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
import json
import os
import re
import subprocess
import sys
import time
import tomllib
import uuid
from collections import Counter, defaultdict
from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlsplit

duckdb: Any = importlib.import_module("duckdb")

DEFAULT_SCHEMA_VERSION = 4
DEFAULT_GENERATOR_VERSION = "0.5.0"
DEFAULT_PIPELINE_MANIFEST_PATH = "data/meta.toml"
DEFAULT_CONTENT_PREFIXES = (
    "docs/",
    "website/content/",
    "website/static/",
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


@dataclass
class PipelineConfig:
    schema_version: int
    generator_version: str
    database_path: Path
    build_output_dir: Path
    commit_limit: int
    term_limit: int
    content_prefixes: tuple[str, ...]
    content_excludes: tuple[str, ...]
    dashboard_manifest_path: Path
    dashboard_manifest_output_path: Path
    zorto_command: list[str]
    public_command: str
    forbidden_regex: tuple[str, ...]


@dataclass
class PipelineStep:
    step_name: str
    step_kind: str
    started_at: str
    finished_at: str
    duration_ms: int
    status: str
    input_count: int | None = None
    output_count: int | None = None
    command: str | None = None
    detail: str | None = None
    warning: str | None = None
    error: str | None = None


class PipelineRecorder:
    def __init__(self) -> None:
        self.steps: list[PipelineStep] = []

    def run(
        self,
        step_name: str,
        step_kind: str,
        fn: Callable[[], Any],
        *,
        input_count: int | None = None,
        output_count: Callable[[Any], int | None] | None = None,
        command: str | None = None,
        detail: str | None = None,
    ) -> Any:
        started = now_iso()
        started_monotonic = time.monotonic()
        status = "success"
        error = None
        result: Any
        try:
            result = fn()
        except Exception as exc:
            status = "error"
            error = str(exc)
            raise
        finally:
            finished = now_iso()
            duration_ms = round((time.monotonic() - started_monotonic) * 1000)
            rows = None
            if (
                status == "success"
                and output_count is not None
                and "result" in locals()
            ):
                rows = output_count(result)
            self.steps.append(
                PipelineStep(
                    step_name=step_name,
                    step_kind=step_kind,
                    started_at=started,
                    finished_at=finished,
                    duration_ms=duration_ms,
                    status=status,
                    input_count=input_count,
                    output_count=rows,
                    command=command,
                    detail=detail,
                    error=error,
                )
            )
        return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate zorto.dev metadata database")
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--website-dir", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--build-output", type=Path)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    website_dir = args.website_dir.resolve()
    config = load_pipeline_config(
        repo_root,
        website_dir,
        args.manifest or website_dir / DEFAULT_PIPELINE_MANIFEST_PATH,
    )
    output = (args.output or config.database_path).resolve()
    build_output = (args.build_output or config.build_output_dir).resolve()
    config.database_path = output
    config.build_output_dir = build_output

    recorder = PipelineRecorder()
    previous_runs: list[BuildRun] = recorder.run(
        "read_previous_build_runs",
        "duckdb",
        lambda: read_previous_build_runs(output),
        output_count=len,
        detail=rel(repo_root, output) if output.exists() else "no previous database",
    )
    build_run: BuildRun = recorder.run(
        "run_zorto_build",
        "command",
        lambda: run_zorto_build(repo_root, website_dir, config),
        output_count=lambda _run: 1,
        command=render_public_command(repo_root, config),
    )
    build_outputs: list[tuple[str, int, str, str, str]] = recorder.run(
        "collect_build_outputs",
        "filesystem",
        lambda: collect_build_outputs(build_output),
        output_count=len,
        detail=rel(repo_root, build_output),
    )

    tmp = output.with_name(f".{output.name}.{uuid.uuid4().hex}.tmp")
    cleanup_duckdb_files(tmp)
    output.parent.mkdir(parents=True, exist_ok=True)

    con = duckdb.connect(str(tmp))
    try:
        write_database(
            con,
            repo_root,
            config,
            recorder,
            previous_runs + [build_run],
            build_outputs,
        )
        con.close()
        recorder.run(
            "privacy_scan",
            "guard",
            lambda: run_privacy_checks(repo_root, tmp, config),
            output_count=lambda findings: 0 if findings is None else len(findings),
            detail=rel(repo_root, output),
        )
        refresh_pipeline_steps(tmp, recorder.steps)
        cleanup_duckdb_files(output)
        os.replace(tmp, output)
    finally:
        try:
            con.close()
        except Exception:
            pass
        cleanup_duckdb_files(tmp)

    print(f"wrote {rel(repo_root, output)}")
    manifest_output = recorder.run(
        "emit_dashboard_manifest",
        "config",
        lambda: emit_dashboard_manifest(config),
        output_count=lambda path: 1 if path else 0,
    )
    if manifest_output:
        print(f"wrote {rel(repo_root, manifest_output)}")
    return 0


def load_pipeline_config(
    repo_root: Path, website_dir: Path, manifest_path: Path
) -> PipelineConfig:
    manifest = read_toml(manifest_path) if manifest_path.exists() else {}
    pipeline = manifest.get("pipeline", {})
    limits = manifest.get("limits", {})
    content = manifest.get("content", {})
    dashboard = manifest.get("dashboard_manifest", {})
    zorto_build = manifest.get("zorto_build", {})
    privacy = manifest.get("privacy", {})

    schema_version = int(pipeline.get("schema_version", DEFAULT_SCHEMA_VERSION))
    generator_version = str(
        pipeline.get("generator_version", DEFAULT_GENERATOR_VERSION)
    )
    database_path = repo_path(
        repo_root, str(pipeline.get("database_path", "website/static/data/site.ddb"))
    )
    build_output_dir = repo_path(
        repo_root,
        str(pipeline.get("build_output_dir", "website/target/zorto-meta-public")),
    )
    command = [
        str(part)
        for part in zorto_build.get(
            "command",
            [
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
                "target/zorto-meta-public",
            ],
        )
    ]
    public_command = str(
        zorto_build.get(
            "public_command",
            "cargo run -p zorto -- --root website --sandbox . build --output "
            "target/zorto-meta-public",
        )
    )

    return PipelineConfig(
        schema_version=schema_version,
        generator_version=generator_version,
        database_path=database_path,
        build_output_dir=build_output_dir,
        commit_limit=int(limits.get("commit_limit", 250)),
        term_limit=int(limits.get("term_limit", 500)),
        content_prefixes=tuple(
            str(prefix) for prefix in content.get("prefixes", DEFAULT_CONTENT_PREFIXES)
        ),
        content_excludes=tuple(str(path) for path in content.get("excludes", [])),
        dashboard_manifest_path=repo_path(
            repo_root, str(dashboard.get("source", "website/data/analytics.toml"))
        ),
        dashboard_manifest_output_path=repo_path(
            repo_root,
            str(
                dashboard.get("output", "website/static/data/analytics-dashboard.json")
            ),
        ),
        zorto_command=command,
        public_command=public_command,
        forbidden_regex=tuple(
            str(pattern) for pattern in privacy.get("forbidden_regex", [])
        ),
    )


def emit_dashboard_manifest(config: PipelineConfig) -> Path | None:
    source = config.dashboard_manifest_path
    if not source.exists():
        return None

    manifest = read_toml(source)
    try:
        manifest["source_path"] = rel(source.parent.parent, source)
    except ValueError:
        manifest["source_path"] = source.name
    manifest["generated_at"] = now_iso()

    output = config.dashboard_manifest_output_path
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_name(f".{output.name}.{uuid.uuid4().hex}.tmp")
    tmp.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    os.replace(tmp, output)
    return output


def write_database(
    con: Any,
    repo_root: Path,
    config: PipelineConfig,
    recorder: PipelineRecorder,
    build_runs: list[BuildRun],
    build_outputs: list[tuple[str, int, str, str, str]],
) -> None:
    create_schema(con)
    recorder.run(
        "insert_meta_info",
        "duckdb",
        lambda: insert_meta_info(con, config),
        output_count=lambda _result: 1,
    )
    recorder.run(
        "insert_repo_snapshot",
        "git",
        lambda: insert_repo_snapshot(con, repo_root),
        output_count=lambda _result: 1,
    )
    commits: list[Commit] = recorder.run(
        "collect_commits",
        "git",
        lambda: collect_commits(repo_root, config),
        output_count=len,
    )
    recorder.run(
        "insert_commits",
        "duckdb",
        lambda: insert_commits(con, commits),
        input_count=len(commits),
        output_count=lambda _result: len(commits),
    )
    recorder.run(
        "insert_commit_daily",
        "duckdb",
        lambda: insert_commit_daily(con, commits),
        input_count=len(commits),
        output_count=lambda rows: rows,
    )
    recorder.run(
        "insert_packages",
        "duckdb",
        lambda: insert_packages(con, repo_root),
        output_count=lambda rows: rows,
    )
    content_files: list[ContentFile] = recorder.run(
        "collect_content_files",
        "filesystem",
        lambda: collect_content_files(repo_root, config),
        output_count=len,
    )
    recorder.run(
        "insert_content_files",
        "duckdb",
        lambda: insert_content_files(con, content_files),
        input_count=len(content_files),
        output_count=lambda _result: len(content_files),
    )
    recorder.run(
        "insert_content_terms",
        "duckdb",
        lambda: insert_content_terms(con, content_files, config),
        input_count=len(content_files),
        output_count=lambda rows: rows,
    )
    recorder.run(
        "insert_search_pages",
        "duckdb",
        lambda: insert_search_pages(con, content_files),
        input_count=len(content_files),
        output_count=lambda rows: rows,
    )
    recorder.run(
        "insert_content_links",
        "duckdb",
        lambda: insert_content_links(con, repo_root, content_files, build_outputs),
        input_count=len(content_files),
        output_count=lambda rows: rows,
    )
    recorder.run(
        "insert_build_runs",
        "duckdb",
        lambda: insert_build_runs(con, build_runs),
        input_count=len(build_runs),
        output_count=lambda _result: len(build_runs),
    )
    recorder.run(
        "insert_build_outputs",
        "duckdb",
        lambda: con.executemany(
            "INSERT INTO build_outputs VALUES (?, ?, ?, ?, ?)", build_outputs
        ),
        input_count=len(build_outputs),
        output_count=lambda _result: len(build_outputs),
    )
    insert_pipeline_steps(con, recorder.steps)
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
        CREATE TABLE search_pages (
            path VARCHAR NOT NULL,
            title VARCHAR NOT NULL,
            url VARCHAR NOT NULL,
            description VARCHAR,
            content VARCHAR NOT NULL,
            title_lower VARCHAR NOT NULL,
            description_lower VARCHAR NOT NULL,
            content_lower VARCHAR NOT NULL,
            kind VARCHAR NOT NULL,
            word_count INTEGER NOT NULL
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
        CREATE TABLE pipeline_steps (
            step_name VARCHAR NOT NULL,
            step_kind VARCHAR NOT NULL,
            started_at TIMESTAMP NOT NULL,
            finished_at TIMESTAMP NOT NULL,
            duration_ms INTEGER NOT NULL,
            status VARCHAR NOT NULL,
            input_count INTEGER,
            output_count INTEGER,
            command VARCHAR,
            detail VARCHAR,
            warning VARCHAR,
            error VARCHAR
        );
        """
    )


def insert_meta_info(con: Any, config: PipelineConfig) -> None:
    con.execute(
        "INSERT INTO meta_info VALUES (?, ?, ?, ?)",
        [
            config.schema_version,
            now_iso(),
            config.generator_version,
            duckdb.__version__,
        ],
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


def collect_commits(repo_root: Path, config: PipelineConfig) -> list[Commit]:
    output = git(
        repo_root,
        "log",
        f"--max-count={config.commit_limit}",
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


def insert_commit_daily(con: Any, commits: list[Commit]) -> int:
    daily: dict[str, dict[str, int]] = defaultdict(
        lambda: {"commit_count": 0, "file_count": 0, "additions": 0, "deletions": 0}
    )
    for commit in commits:
        day = commit.committed_at[:10]
        daily[day]["commit_count"] += 1
        daily[day]["file_count"] += commit.file_count
        daily[day]["additions"] += commit.additions
        daily[day]["deletions"] += commit.deletions
    rows = [
        (
            day,
            vals["commit_count"],
            vals["file_count"],
            vals["additions"],
            vals["deletions"],
        )
        for day, vals in sorted(daily.items())
    ]
    con.executemany(
        "INSERT INTO commit_daily VALUES (?, ?, ?, ?, ?)",
        rows,
    )
    return len(rows)


def insert_packages(con: Any, repo_root: Path) -> int:
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
    return len(rows)


def collect_content_files(repo_root: Path, config: PipelineConfig) -> list[ContentFile]:
    rows: list[ContentFile] = []
    for rel_path in git(repo_root, "ls-files").splitlines():
        if (
            not rel_path.startswith(config.content_prefixes)
            or rel_path in config.content_excludes
        ):
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


def insert_content_terms(
    con: Any, files: list[ContentFile], config: PipelineConfig
) -> int:
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
        for term, count in counts.most_common(config.term_limit)
    ]
    con.executemany("INSERT INTO content_terms VALUES (?, ?, ?)", rows)
    return len(rows)


def insert_search_pages(con: Any, files: list[ContentFile]) -> int:
    rows: list[tuple[str, str, str, str, str, str, str, str, str, int]] = []
    for file in files:
        if file.kind not in {"content", "docs"} or not file.text:
            continue
        url = search_url_for_path(file.path)
        if url is None:
            continue
        title = file.title or title_from_path(file.path)
        description = extract_description(file.text)
        content = search_text(file.text)
        rows.append(
            (
                file.path,
                title,
                url,
                description,
                content,
                title.lower(),
                (description or "").lower(),
                content.lower(),
                file.kind,
                file.word_count,
            )
        )
    con.executemany(
        "INSERT INTO search_pages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", rows
    )
    return len(rows)


def insert_content_links(
    con: Any,
    repo_root: Path,
    files: list[ContentFile],
    build_outputs: list[tuple[str, int, str, str, str]],
) -> int:
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
    return len(rows)


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


def insert_pipeline_steps(con: Any, steps: list[PipelineStep]) -> None:
    con.executemany(
        "INSERT INTO pipeline_steps VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                step.step_name,
                step.step_kind,
                step.started_at,
                step.finished_at,
                step.duration_ms,
                step.status,
                step.input_count,
                step.output_count,
                step.command,
                step.detail,
                step.warning,
                step.error,
            )
            for step in steps
        ],
    )


def refresh_pipeline_steps(path: Path, steps: list[PipelineStep]) -> None:
    con = duckdb.connect(str(path))
    try:
        con.execute("DELETE FROM pipeline_steps")
        insert_pipeline_steps(con, steps)
        con.execute("CHECKPOINT")
    finally:
        con.close()


def run_zorto_build(
    repo_root: Path, website_dir: Path, config: PipelineConfig
) -> BuildRun:
    build_output = config.build_output_dir
    build_output.mkdir(parents=True, exist_ok=True)
    started = now_iso()
    started_monotonic = time.monotonic()
    env = os.environ.copy()
    env["VIRTUAL_ENV"] = str(website_dir / ".venv")
    subprocess.run(
        render_command(repo_root, config.zorto_command, config.build_output_dir),
        cwd=repo_root,
        check=True,
        env=env,
    )
    finished = now_iso()
    return BuildRun(
        run_id=uuid.uuid4().hex,
        started_at=started,
        finished_at=finished,
        duration_ms=round((time.monotonic() - started_monotonic) * 1000),
        status="success",
        zorto_version=zorto_version(repo_root),
        command=render_public_command(repo_root, config),
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


def run_privacy_checks(
    repo_root: Path, database_path: Path, config: PipelineConfig
) -> list[str]:
    findings: list[str] = []
    content = database_path.read_bytes().decode("latin-1", errors="ignore")
    default_patterns = [
        re.escape(str(repo_root)),
        r"/Users/",
        r"/private/",
        r"VIRTUAL_ENV",
        r"\bHOME=",
        r"\bPATH=",
        r"GITHUB_TOKEN",
        r"ghp_[A-Za-z0-9_]+",
        r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}",
    ]
    for pattern in [*default_patterns, *config.forbidden_regex]:
        if re.search(pattern, content):
            findings.append(pattern)

    allowed_untracked = {
        rel(repo_root, config.database_path),
        rel(
            repo_root,
            config.database_path.with_suffix(config.database_path.suffix + ".wal"),
        ),
        rel(repo_root, config.dashboard_manifest_output_path),
    }
    for untracked in git(
        repo_root, "ls-files", "--others", "--exclude-standard"
    ).splitlines():
        if untracked in allowed_untracked:
            continue
        if untracked and untracked in content:
            findings.append(f"untracked path leaked: {untracked}")

    if findings:
        raise RuntimeError("privacy scan failed: " + "; ".join(findings[:6]))
    return findings


def git(repo_root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=repo_root, text=True).rstrip(
        "\n"
    )


def read_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def repo_path(repo_root: Path, path: str) -> Path:
    candidate = Path(path)
    if candidate.is_absolute():
        return candidate
    return repo_root / candidate


def render_command(
    repo_root: Path, command: list[str], build_output: Path
) -> list[str]:
    return [
        part.replace("{build_output_dir}", rel(repo_root, build_output))
        for part in command
    ]


def render_public_command(repo_root: Path, config: PipelineConfig) -> str:
    return config.public_command.replace(
        "{build_output_dir}", rel(repo_root, config.build_output_dir)
    )


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


def extract_description(text: str) -> str:
    if text.startswith("+++\n"):
        end = text.find("\n+++", 4)
        if end != -1:
            try:
                frontmatter = tomllib.loads(text[4:end])
                description = frontmatter.get("description")
                if isinstance(description, str):
                    return description
            except tomllib.TOMLDecodeError:
                pass
    return ""


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


def search_text(text: str) -> str:
    text = strip_code(strip_frontmatter(text))
    text = re.sub(r"!\[[^\]]*\]\([^)]+\)", " ", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = re.sub(r"\{\{.*?\}\}", " ", text, flags=re.DOTALL)
    text = re.sub(r"\{%.*?%\}", " ", text, flags=re.DOTALL)
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"^[#>\-\*\s]+", " ", text, flags=re.MULTILINE)
    text = re.sub(r"[`*_~|]+", " ", text)
    return re.sub(r"\s+", " ", text).strip()


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


def search_url_for_path(rel_path: str) -> str | None:
    if rel_path == "website/content/_index.md":
        return "/"
    if rel_path.startswith("website/content/presentations/intro-to-zorto/"):
        if rel_path != "website/content/presentations/intro-to-zorto/_index.md":
            return None
    if rel_path.startswith("website/content/"):
        local = rel_path.removeprefix("website/content/")
        if local == "_index.md":
            return "/"
        if local.endswith("/_index.md"):
            return "/" + local.removesuffix("/_index.md") + "/"
        if local.endswith(".md"):
            return "/" + local.removesuffix(".md") + "/"
    if rel_path == "docs/README.md":
        return "/docs/"
    if rel_path.startswith("docs/"):
        local = rel_path.removeprefix("docs/")
        if local == "README.md":
            return "/docs/"
        if local.endswith("/README.md"):
            return "/docs/" + local.removesuffix("/README.md") + "/"
        if local.endswith(".md"):
            return "/docs/" + local.removesuffix(".md") + "/"
    return None


def title_from_path(rel_path: str) -> str:
    stem = Path(rel_path).stem
    if stem in {"README", "_index"}:
        stem = Path(rel_path).parent.name or "Home"
    return stem.replace("-", " ").replace("_", " ").title()


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
