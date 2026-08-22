# Contributing

Contributions that improve fairness, correctness, interoperability, or reproducibility are welcome.

## Ground rules

- Add a format only with a documented semantic mapping, a credible maintained implementation, round-trip coverage, and an honest explanation of ecosystem support.
- Do not tune one format’s workload, framing, compression input, or sample independently of the others.
- Distinguish a base format from a project-specific profile and from an implementation optimization.
- Preserve arbitrary NIP-01 tag strings exactly.
- Do not update benchmark tables by hand without retaining the command, corpus fingerprint, host metadata, and machine-readable result.
- Do not add or replace corpus data without documented source, collection method, dates, transformations, license/redistribution basis, and privacy review.

## Local checks

Install `capnp`, `protoc`, and `flatc`, then run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run --release --example generate_vectors

python3 -m venv .venv-interop
.venv-interop/bin/pip install -r scripts/interop-requirements.txt
.venv-interop/bin/python scripts/verify_python_interop.py
```

For codec or measurement changes, run `allocation_report` and then regenerate `results/latest.json` and `results/latest.md` from an idle machine. Explain any material movement. Use Criterion for performance claims; the `public_report` runner is a reproducible multi-metric snapshot, not a substitute for confidence intervals. Publication-profile Criterion results are exported from the raw estimate files rather than transcribed:

```bash
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench zero_copy -- --noplot
BINOSTR_PUBLICATION_BENCH=1 cargo bench --bench validated -- --noplot
python3 scripts/export_criterion.py
python3 scripts/record_criterion_run.py record my-host-YYYYMMDD run-1
```

Repeat both benchmark commands and the export before recording `run-2` and `run-3` under the same new campaign label. The recorder refuses to combine different commits, dirty states, toolchains, hosts, corpus fingerprints, seeds, or Criterion profiles. Retain `target/criterion` locally when interpreting close results. Per-run confidence intervals and the three-run range are evidence, not proof that close ordering transfers to another host.

## Pull requests

Describe:

1. the methodology or behavior changed;
2. why the comparison remains semantically fair;
3. correctness, robustness, and interoperability evidence;
4. benchmark host, corpus fingerprint, commands, and Criterion confidence intervals when making a performance claim; and
5. known limitations or target-language gaps.

Keep generated schemas and vectors in the same change as the code that produces them.
