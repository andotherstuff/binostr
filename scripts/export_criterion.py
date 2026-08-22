#!/usr/bin/env python3
"""Export tracked JSON and Markdown snapshots from Criterion estimate files."""

from __future__ import annotations

import datetime as dt
import json
import platform
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRITERION = ROOT / "target" / "criterion"
RESULTS = ROOT / "results"
EVENT_COUNT = 1_000
SEED = 4_776_434_982_410_539_521
FINGERPRINT = "a05ceb17de26aca15c5b284652a6e55188b0c5a135fb89839459d9d93ae492f5"
COMMAND_PREFIX = "BINOSTR_PUBLICATION_BENCH=1 cargo bench"


@dataclass(frozen=True)
class Format:
    key: str
    criterion: str
    label: str


FORMATS = (
    Format("json", "json", "JSON"),
    Format("cbor_packed", "cbor_packed", "CBOR packed"),
    Format("msgpack", "msgpack", "MessagePack"),
    Format("flatbuffers", "flatbuffers", "FlatBuffers"),
    Format("flexbuffers", "flexbuffers", "FlexBuffers"),
    Format("avro", "avro", "Avro binary datum"),
    Format("bson", "bson", "BSON"),
    Format("thrift_compact", "thrift", "Thrift Compact"),
    Format("beve", "beve", "BEVE emerging reference"),
    Format("bincode", "bincode", "bincode reference"),
    Format("postcard", "postcard", "postcard reference"),
    Format("proto_binary", "proto_bin", "Protocol Buffers binary"),
    Format("capnp_packed", "capnp_pk", "Cap’n Proto packed custom profile"),
    Format("dannypack", "dannypack", "DannyPack custom reference"),
    Format("notepack", "notepack", "Notepack custom reference"),
)

SELECTIVE = (
    ("notepack/selective_checked", "notepack_selective_checked", "Notepack checked selective kind + pubkey", "selected-field parser checks"),
    ("flatbuffers/selective_preverified", "flatbuffers_selective_preverified", "FlatBuffers selective, exact buffer preverified", "exact immutable buffer verified earlier"),
    ("flexbuffers/selective_checked", "flexbuffers_selective_checked", "FlexBuffers checked selective kind + pubkey", "root, vector, bounds, and selected types checked; skipped subtrees not fully verified"),
    ("capnp/selective_checked", "capnp_selective_checked", "Cap’n Proto selective checked", "message reader and selected fields checked"),
    ("beve/selective_preverified", "beve_selective_preverified", "BEVE selective, exact buffer preverified", "exact immutable buffer verified earlier; selected paths checked"),
    ("beve/selective_verify_each", "beve_selective_verify_each", "BEVE selective, verify each read", "whole value verified before selected paths"),
    ("flatbuffers/selective_verify_each", "flatbuffers_selective_verify_each", "FlatBuffers selective, verify each read", "whole buffer verified before selected fields"),
)


def command(*args: str) -> str:
    try:
        return subprocess.run(args, cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def metadata(group: str) -> dict:
    status = command("git", "status", "--porcelain")
    cpu = command("sysctl", "-n", "machdep.cpu.brand_string")
    host = f"{platform.system()} {platform.release()} {platform.machine()}"
    if cpu != "unknown":
        host += f"; {cpu}"
    estimate_files = list((CRITERION / group).glob("*/new/estimates.json"))
    if not estimate_files:
        raise FileNotFoundError(f"no Criterion estimates found under {CRITERION / group}")
    completed = dt.datetime.fromtimestamp(
        max(path.stat().st_mtime for path in estimate_files), dt.timezone.utc
    )
    return {
        "exported_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "benchmark_estimates_completed_utc": completed.replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "git_commit": command("git", "rev-parse", "HEAD"),
        "git_dirty": bool(status and status != "unknown"),
        "rustc": command("rustc", "-V"),
        "host": host,
        "run_count": 1,
        "power_and_thermal_state": "not recorded",
    }


def estimate(group: str, benchmark: str) -> dict[str, float]:
    path = CRITERION / group / benchmark / "new" / "estimates.json"
    raw = json.loads(path.read_text())
    value = raw.get("slope") or raw["mean"]
    interval = value["confidence_interval"]
    return {
        "lower": interval["lower_bound"] / EVENT_COUNT,
        "point": value["point_estimate"] / EVENT_COUNT,
        "upper": interval["upper_bound"] / EVENT_COUNT,
    }


def criterion_profile() -> dict:
    return {
        "samples": 100,
        "warmup_ms": 5_000,
        "measurement_seconds": 10,
        "confidence": 0.95,
        "noise_threshold": 0.01,
    }


def common(command_line: str, run_metadata: dict) -> dict:
    return {
        "schema_version": 2,
        "metadata": run_metadata,
        "command": command_line,
        "event_count": EVENT_COUNT,
        "seed": SEED,
        "corpus_id_sha256": FINGERPRINT,
        "criterion": criterion_profile(),
        "units": "nanoseconds_per_event",
    }


def ms(interval: dict[str, float]) -> str:
    return f"{interval['lower'] / 1_000:.4f}–{interval['upper'] / 1_000:.4f} ms"


def duration(interval: dict[str, float]) -> str:
    # For 1,000 events, the numeric ns/event value is also the total duration in µs.
    low, high = interval["lower"], interval["upper"]
    if high < 1_000:
        return f"{low:.2f}–{high:.2f} µs"
    return f"{low / 1_000:.4f}–{high / 1_000:.4f} ms"


def export_validated() -> None:
    command_line = f"{COMMAND_PREFIX} --bench validated -- --noplot"
    run_metadata = metadata("validated_ingest")
    report = common(command_line, run_metadata)
    measurements = []
    for fmt in FORMATS:
        measurements.append({
            "key": fmt.key,
            "decode_id": estimate("validated_ingest", f"{fmt.criterion}_decode_id"),
            "validated": estimate("validated_ingest", f"{fmt.criterion}_decode_id_signature"),
        })
    report["measurements"] = measurements
    (RESULTS / "validated-ingest.json").write_text(json.dumps(report, indent=2) + "\n")

    lines = [
        "# Validated-ingest Criterion snapshot",
        "",
        f"Criterion run on {EVENT_COUNT:,} events: seed `{SEED}`, event-ID fingerprint `{FINGERPRINT}`.",
        "",
        f"Criterion estimates completed `{run_metadata['benchmark_estimates_completed_utc']}` and "
        f"were exported `{run_metadata['exported_utc']}` from commit `{run_metadata['git_commit']}` "
        f"with a {'dirty' if run_metadata['git_dirty'] else 'clean'} worktree, using "
        f"`{run_metadata['rustc']}` on `{run_metadata['host']}`. This is one publication-profile "
        "run; power and thermal state were not recorded.",
        "",
        f"Command: `{command_line}`. The publication profile uses 100 samples, a five-second "
        "warm-up, a ten-second target measurement, and 95% confidence intervals. Each interval "
        "is the time for 1,000 events; divide milliseconds by 1,000 to obtain microseconds/event.",
        "",
        "| Format | Bounded decode + ID | Bounded decode + ID + signature |",
        "|---|---:|---:|",
    ]
    for fmt, measurement in zip(FORMATS, measurements):
        lines.append(f"| {fmt.label} | {ms(measurement['decode_id'])} | {ms(measurement['validated'])} |")
    lines.extend([
        "",
        "Criterion’s raw sample and outlier evidence remains under `target/criterion`; retain it "
        "when making a close comparison. Codec differences remain visible after decode plus ID, "
        "but BIP-340 verification substantially narrows their share of accepted-event cost.",
    ])
    (RESULTS / "validated-ingest.md").write_text("\n".join(lines) + "\n")


def export_selective() -> None:
    command_line = f"{COMMAND_PREFIX} --bench zero_copy -- --noplot"
    run_metadata = metadata("read_kind_and_pubkey")
    report = common(command_line, run_metadata)
    measurements = []
    for path, criterion_name, label, contract in SELECTIVE:
        measurements.append({
            "path": path,
            "label": label,
            "contract": contract,
            "interval": estimate("read_kind_and_pubkey", criterion_name),
        })
    for fmt in FORMATS:
        measurements.append({
            "path": f"{fmt.key}/full_materialize",
            "label": f"{fmt.label} full materialization",
            "contract": "complete owned event",
            "interval": estimate("read_kind_and_pubkey", f"{fmt.criterion}_full_materialize"),
        })
    report["measurements"] = measurements
    (RESULTS / "selective-access.json").write_text(json.dumps(report, indent=2) + "\n")

    ordered = sorted(measurements, key=lambda item: item["interval"]["point"])
    lines = [
        "# Selective-access snapshot",
        "",
        f"Criterion run on the same {EVENT_COUNT:,}-event corpus as `latest.json`: seed `{SEED}`, "
        f"event-ID fingerprint `{FINGERPRINT}`.",
        "",
        f"Criterion estimates completed `{run_metadata['benchmark_estimates_completed_utc']}` and "
        f"were exported `{run_metadata['exported_utc']}` from commit `{run_metadata['git_commit']}` "
        f"with a {'dirty' if run_metadata['git_dirty'] else 'clean'} worktree, using "
        f"`{run_metadata['rustc']}` on `{run_metadata['host']}`. This is one publication-profile "
        "run; power and thermal state were not recorded.",
        "",
        f"Command: `{command_line}`. The publication profile uses 100 samples, a five-second "
        "warm-up, a ten-second target measurement, and 95% confidence intervals. Times below are "
        "for 1,000 events; the last column normalizes the midpoint per event.",
        "",
        "| Path | Safety/materialization contract | Criterion interval | Approx. ns/event |",
        "|---|---|---:|---:|",
    ]
    for measurement in ordered:
        interval = measurement["interval"]
        lines.append(
            f"| {measurement['label']} | {measurement['contract']} | {duration(interval)} | "
            f"{interval['point']:,.1f} |"
        )
    lines.extend([
        "",
        "Selective paths are not equivalent safety contracts. “Preverified” means the exact "
        "immutable buffer was fully verified earlier; that cost must be paid once at an ingress "
        "or trust boundary. Checked selected-field paths validate navigation, bounds, and selected "
        "types but may not validate skipped subtrees. Full materialization creates the complete "
        "owned event. The Cap’n Proto profile also contains custom packed byte blobs.",
    ])
    (RESULTS / "selective-access.md").write_text("\n".join(lines) + "\n")


def main() -> None:
    RESULTS.mkdir(exist_ok=True)
    export_validated()
    export_selective()
    print("Wrote Criterion snapshots under results/")


if __name__ == "__main__":
    main()
