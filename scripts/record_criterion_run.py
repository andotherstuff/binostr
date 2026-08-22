#!/usr/bin/env python3
"""Record one exported Criterion run and summarize cross-run repeatability."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "results"
RUNS = RESULTS / "criterion-runs"
LABEL_RE = re.compile(r"^[a-zA-Z0-9][a-zA-Z0-9_-]*$")


def load(path: Path) -> dict:
    return json.loads(path.read_text())


def checked_name(value: str, description: str) -> str:
    if not LABEL_RE.fullmatch(value):
        raise SystemExit(
            f"{description} may contain only letters, digits, underscores, and hyphens"
        )
    return value


def record(campaign: str, label: str) -> Path:
    campaign = checked_name(campaign, "campaign")
    label = checked_name(label, "run label")
    RUNS.mkdir(parents=True, exist_ok=True)
    destination = RUNS / f"{campaign}--{label}.json"
    if destination.exists():
        raise SystemExit(f"refusing to overwrite existing run: {destination}")

    validated = load(RESULTS / "validated-ingest.json")
    selective = load(RESULTS / "selective-access.json")
    for report in (validated, selective):
        if report.get("schema_version") != 2:
            raise SystemExit("unsupported exported Criterion schema")
    for field in ("event_count", "seed", "corpus_id_sha256", "criterion", "units"):
        if validated[field] != selective[field]:
            raise SystemExit(f"exported suites disagree on {field}")
    for field in ("git_commit", "git_dirty", "rustc", "host"):
        if validated["metadata"][field] != selective["metadata"][field]:
            raise SystemExit(f"exported suites disagree on metadata.{field}")

    snapshot = {
        "schema_version": 1,
        "campaign": campaign,
        "label": label,
        "recorded_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "validated_ingest": validated,
        "selective_access": selective,
    }
    destination.write_text(json.dumps(snapshot, indent=2) + "\n")
    return destination


def ensure_same_run_contract(runs: list[dict]) -> None:
    first = runs[0]
    if any(run["campaign"] != first["campaign"] for run in runs):
        raise SystemExit("cannot aggregate runs from different campaigns")
    for suite in ("validated_ingest", "selective_access"):
        expected = first[suite]
        for run in runs[1:]:
            actual = run[suite]
            for field in ("event_count", "seed", "corpus_id_sha256", "criterion", "units"):
                if actual[field] != expected[field]:
                    raise SystemExit(
                        f"cannot aggregate runs with different {suite}.{field} values"
                    )
            for field in ("git_commit", "git_dirty", "rustc", "host"):
                if actual["metadata"][field] != expected["metadata"][field]:
                    raise SystemExit(
                        f"cannot aggregate runs with different {suite}.metadata.{field} values"
                    )


def series(values: list[float]) -> dict:
    median = statistics.median(values)
    return {
        "points": values,
        "median": median,
        "minimum": min(values),
        "maximum": max(values),
        "relative_range_percent": (max(values) - min(values)) / median * 100,
    }


def aggregate(runs: list[dict]) -> dict:
    ensure_same_run_contract(runs)
    first = runs[0]
    by_label = [run["label"] for run in runs]

    validated = []
    for index, measurement in enumerate(first["validated_ingest"]["measurements"]):
        key = measurement["key"]
        if any(
            run["validated_ingest"]["measurements"][index]["key"] != key
            for run in runs
        ):
            raise SystemExit("validated-ingest format ordering differs between runs")
        validated.append(
            {
                "key": key,
                "decode_id": series(
                    [
                        run["validated_ingest"]["measurements"][index]["decode_id"][
                            "point"
                        ]
                        for run in runs
                    ]
                ),
                "validated": series(
                    [
                        run["validated_ingest"]["measurements"][index]["validated"][
                            "point"
                        ]
                        for run in runs
                    ]
                ),
            }
        )

    selective = []
    for index, measurement in enumerate(first["selective_access"]["measurements"]):
        path = measurement["path"]
        if any(
            run["selective_access"]["measurements"][index]["path"] != path
            for run in runs
        ):
            raise SystemExit("selective-access path ordering differs between runs")
        selective.append(
            {
                "path": path,
                "label": measurement["label"],
                "contract": measurement["contract"],
                "timing": series(
                    [
                        run["selective_access"]["measurements"][index]["interval"][
                            "point"
                        ]
                        for run in runs
                    ]
                ),
            }
        )

    return {
        "schema_version": 1,
        "generated_utc": max(run["recorded_utc"] for run in runs),
        "campaign": first["campaign"],
        "run_count": len(runs),
        "run_labels": by_label,
        "event_count": first["validated_ingest"]["event_count"],
        "seed": first["validated_ingest"]["seed"],
        "corpus_id_sha256": first["validated_ingest"]["corpus_id_sha256"],
        "criterion": first["validated_ingest"]["criterion"],
        "units": "nanoseconds_per_event",
        "metadata": {
            field: first["validated_ingest"]["metadata"][field]
            for field in ("git_commit", "git_dirty", "rustc", "host")
        },
        "validated_ingest": validated,
        "selective_access": selective,
    }


def format_runs(values: list[float]) -> str:
    return ", ".join(f"{value / 1_000:.3f}" for value in values)


def markdown(report: dict) -> str:
    runs = report["run_count"]
    lines = [
        "# Criterion repeatability across independent runs",
        "",
        f"This report compares {runs} independent publication-profile Criterion runs on "
        f"{report['event_count']:,} events. Values are point estimates from each run; the "
        "per-run JSON files retain each Criterion confidence interval.",
        "",
        f"Campaign: `{report['campaign']}`. Commit: `{report['metadata']['git_commit']}` "
        f"({'dirty worktree' if report['metadata']['git_dirty'] else 'clean worktree'}). "
        f"Toolchain: `{report['metadata']['rustc']}`. Host: `{report['metadata']['host']}`.",
        "",
        f"Seed: `{report['seed']}`. Event-ID fingerprint: "
        f"`{report['corpus_id_sha256']}`. Run labels: "
        + ", ".join(f"`{label}`" for label in report["run_labels"])
        + ".",
        "",
        "## Owned decode and validated ingest",
        "",
        "| Format | Owned decode runs (µs/event) | Median | Range | Fully validated runs (µs/event) | Median | Range |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    full_by_key = {
        item["path"].removesuffix("/full_materialize"): item["timing"]
        for item in report["selective_access"]
        if item["path"].endswith("/full_materialize")
    }
    for item in report["validated_ingest"]:
        key = item["key"]
        owned = full_by_key[key]
        validated = item["validated"]
        lines.append(
            f"| {key} | {format_runs(owned['points'])} | "
            f"{owned['median'] / 1_000:.3f} | {owned['relative_range_percent']:.2f}% | "
            f"{format_runs(validated['points'])} | {validated['median'] / 1_000:.3f} | "
            f"{validated['relative_range_percent']:.2f}% |"
        )

    lines.extend(
        [
            "",
            "## Selective paths",
            "",
            "| Path | Safety contract | Runs (ns/event) | Median | Range |",
            "|---|---|---:|---:|---:|",
        ]
    )
    for item in report["selective_access"]:
        if item["path"].endswith("/full_materialize"):
            continue
        timing = item["timing"]
        values = ", ".join(f"{value:.1f}" for value in timing["points"])
        lines.append(
            f"| {item['label']} | {item['contract']} | {values} | "
            f"{timing['median']:.1f} | {timing['relative_range_percent']:.2f}% |"
        )

    lines.extend(
        [
            "",
            "A small cross-run range supports repeatability on this host, but it does not make "
            "close rankings portable across machines or runtimes. Compare safety contracts and "
            "full validation cost before interpreting raw codec ordering.",
        ]
    )
    return "\n".join(lines) + "\n"


def write_aggregate(campaign: str) -> None:
    paths = sorted(RUNS.glob("*.json"))
    if not paths:
        return
    runs = [run for path in paths if (run := load(path))["campaign"] == campaign]
    if not runs:
        raise SystemExit(f"no recorded runs found for campaign {campaign}")
    report = aggregate(runs)
    (RESULTS / "criterion-repeatability.json").write_text(
        json.dumps(report, indent=2) + "\n"
    )
    (RESULTS / "criterion-repeatability.md").write_text(markdown(report))


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="action", required=True)
    record_parser = subparsers.add_parser("record", help="capture the currently exported run")
    record_parser.add_argument(
        "campaign", help="stable campaign label, for example m4max-2026-08"
    )
    record_parser.add_argument("label", help="stable run label, for example run-1")
    aggregate_parser = subparsers.add_parser(
        "aggregate", help="regenerate one campaign's repeatability report"
    )
    aggregate_parser.add_argument("campaign")
    args = parser.parse_args()
    if args.action == "aggregate":
        write_aggregate(args.campaign)
        print(f"Refreshed repeatability reports for campaign {args.campaign}")
        return
    destination = record(args.campaign, args.label)
    write_aggregate(args.campaign)
    print(f"Recorded {destination.relative_to(ROOT)} and refreshed repeatability reports")


if __name__ == "__main__":
    main()
