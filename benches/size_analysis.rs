//! Size analysis benchmark (actually a detailed report generator)

use criterion::{criterion_group, criterion_main, Criterion};

mod common;

use binostr::stats::{compute_aggregate_stats, DistributionAnalysis, Format};

fn size_analysis(c: &mut Criterion) {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              BINOSTR SIZE ANALYSIS REPORT                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let events = common::load_sample(10_000);

    if events.is_empty() {
        println!("No events loaded!");
        return;
    }

    // Distribution analysis
    let dist = DistributionAnalysis::from_events(&events);
    println!("📊 Dataset Summary");
    println!("   Total events: {}", dist.total_events);
    println!(
        "   Average content length: {:.1} bytes",
        dist.avg_content_len
    );
    println!("   Average tag count: {:.1}", dist.avg_tag_count);
    println!();

    // Top kinds
    println!("📈 Top Event Kinds");
    println!("   ┌────────┬─────────┬────────────┐");
    println!("   │ Kind   │ Count   │ Percentage │");
    println!("   ├────────┼─────────┼────────────┤");
    for (kind, count) in dist.top_kinds(10) {
        let pct = 100.0 * count as f64 / dist.total_events as f64;
        println!("   │ {:>6} │ {:>7} │ {:>9.1}% │", kind, count, pct);
    }
    println!("   └────────┴─────────┴────────────┘");
    println!();

    // Size comparison
    println!("📦 Size Comparison (all events)");
    let stats = compute_aggregate_stats(&events);
    let mut sorted: Vec<_> = stats.iter().collect();
    sorted.sort_by(|a, b| a.avg_raw.partial_cmp(&b.avg_raw).unwrap());

    let json_avg = stats
        .iter()
        .find(|s| s.format == Format::Json)
        .map(|s| s.avg_raw)
        .unwrap_or(1.0);

    println!("   ┌──────────────────┬──────────┬──────────┬──────────┬─────────┐");
    println!("   │ Format           │ Avg Raw  │ Avg Gzip │ Avg Zstd │ vs JSON │");
    println!("   ├──────────────────┼──────────┼──────────┼──────────┼─────────┤");
    for stat in &sorted {
        let vs_json = 100.0 * stat.avg_raw / json_avg;
        println!(
            "   │ {:16} │ {:>8.0} │ {:>8.0} │ {:>8.0} │ {:>6.1}% │",
            stat.format.name(),
            stat.avg_raw,
            stat.avg_gzip(),
            stat.avg_zstd(),
            vs_json
        );
    }
    println!("   └──────────────────┴──────────┴──────────┴──────────┴─────────┘");
    println!();

    // Per-kind analysis
    println!("📋 Per-Kind Size Analysis");
    for kind in [0, 1, 3, 7, 30023] {
        let kind_events: Vec<_> = events.iter().filter(|e| e.kind == kind).collect();
        if kind_events.is_empty() {
            continue;
        }

        let kind_name = match kind {
            0 => "Profile",
            1 => "Note",
            3 => "Follow List",
            7 => "Reaction",
            30023 => "Article",
            _ => "Unknown",
        };

        println!(
            "\n   Kind {} ({}) - {} events",
            kind,
            kind_name,
            kind_events.len()
        );

        let mut json_total = 0;
        let mut best_format = Format::Json;
        let mut best_size = usize::MAX;

        let mut sizes: Vec<(Format, usize)> = Vec::new();

        for format in Format::enabled() {
            let total: usize = kind_events
                .iter()
                .map(|e| binostr::stats::serialize(e, format).len())
                .sum();

            if format == Format::Json {
                json_total = total;
            }

            if total < best_size {
                best_size = total;
                best_format = format;
            }

            sizes.push((format, total));
        }

        sizes.sort_by_key(|(_, s)| *s);

        for (format, total) in sizes {
            let avg = total / kind_events.len();
            let vs_json = 100.0 * total as f64 / json_total as f64;
            let marker = if format == best_format {
                " ← best"
            } else {
                ""
            };
            println!(
                "      {:16}: {:>6} bytes avg ({:>5.1}%){}",
                format.name(),
                avg,
                vs_json,
                marker
            );
        }
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Dummy benchmark to satisfy criterion
    let mut group = c.benchmark_group("size_analysis");
    group.bench_function("report", |b| b.iter(|| 1 + 1));
    group.finish();
}

criterion_group! {
    name = benches;
    config = common::auto_criterion();
    targets = size_analysis
}
criterion_main!(benches);
