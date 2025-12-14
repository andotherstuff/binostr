//! Statistics and analysis utilities
//!
//! Tools for analyzing event distributions and serialization metrics.

use std::collections::HashMap;
use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::event::{NostrEvent, SizeCategory, TagCategory};
use crate::{capnp, cbor, dannypack, json, notepack, proto};

/// Serialization format identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Json,
    CborSchemaless,
    CborPacked,
    CborIntKey,
    ProtoString,
    ProtoBinary,
    CapnProto,
    CapnProtoPacked,
    DannyPack,
    Notepack,
}

impl Format {
    pub fn all() -> &'static [Format] {
        &[
            Format::Json,
            Format::CborSchemaless,
            Format::CborPacked,
            Format::CborIntKey,
            Format::ProtoString,
            Format::ProtoBinary,
            Format::CapnProto,
            Format::CapnProtoPacked,
            Format::DannyPack,
            Format::Notepack,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Format::Json => "JSON",
            Format::CborSchemaless => "CBOR Schemaless",
            Format::CborPacked => "CBOR Packed",
            Format::CborIntKey => "CBOR IntKey",
            Format::ProtoString => "Proto String",
            Format::ProtoBinary => "Proto Binary",
            Format::CapnProto => "Cap'n Proto",
            Format::CapnProtoPacked => "Cap'n Packed",
            Format::DannyPack => "DannyPack",
            Format::Notepack => "Notepack",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::CborSchemaless => "cbor_schema",
            Format::CborPacked => "cbor_packed",
            Format::CborIntKey => "cbor_intkey",
            Format::ProtoString => "proto_str",
            Format::ProtoBinary => "proto_bin",
            Format::CapnProto => "capnp",
            Format::CapnProtoPacked => "capnp_pk",
            Format::DannyPack => "dannypack",
            Format::Notepack => "notepack",
        }
    }
}

/// Serialize an event using the specified format
pub fn serialize(event: &NostrEvent, format: Format) -> Vec<u8> {
    match format {
        Format::Json => json::serialize(event),
        Format::CborSchemaless => cbor::schemaless::serialize(event),
        Format::CborPacked => cbor::packed::serialize(event),
        Format::CborIntKey => cbor::intkey::serialize(event),
        Format::ProtoString => proto::string::serialize(event),
        Format::ProtoBinary => proto::binary::serialize(event),
        Format::CapnProto => capnp::serialize_event(event),
        Format::CapnProtoPacked => capnp::serialize_event_packed(event),
        Format::DannyPack => {
            let mut buf = Vec::new();
            dannypack::serialize(event, &mut buf);
            buf
        }
        Format::Notepack => notepack::serialize(event),
    }
}

/// Serialize a batch of events using the specified format
pub fn serialize_batch(events: &[NostrEvent], format: Format) -> Vec<u8> {
    match format {
        Format::Json => json::serialize_batch(events),
        Format::CborSchemaless => cbor::schemaless::serialize_batch(events),
        Format::CborPacked => cbor::packed::serialize_batch(events),
        Format::CborIntKey => cbor::intkey::serialize_batch(events),
        Format::ProtoString => proto::string::serialize_batch(events),
        Format::ProtoBinary => proto::binary::serialize_batch(events),
        Format::CapnProto => capnp::serialize_batch(events),
        Format::CapnProtoPacked => capnp::serialize_batch_packed(events),
        Format::DannyPack => dannypack::serialize_batch(events),
        Format::Notepack => notepack::serialize_batch(events),
    }
}

/// Size statistics for a single format
#[derive(Debug, Clone)]
pub struct SizeStats {
    pub format: Format,
    pub raw_bytes: usize,
    pub gzip_bytes: usize,
    pub zstd_bytes: usize,
}

impl SizeStats {
    pub fn gzip_ratio(&self) -> f64 {
        self.gzip_bytes as f64 / self.raw_bytes as f64
    }

    pub fn zstd_ratio(&self) -> f64 {
        self.zstd_bytes as f64 / self.raw_bytes as f64
    }
}

/// Compute size statistics for an event across all formats
pub fn compute_size_stats(event: &NostrEvent) -> Vec<SizeStats> {
    Format::all()
        .iter()
        .map(|&format| {
            let data = serialize(event, format);
            let raw_bytes = data.len();
            let gzip_bytes = gzip_size(&data);
            let zstd_bytes = zstd_size(&data);

            SizeStats {
                format,
                raw_bytes,
                gzip_bytes,
                zstd_bytes,
            }
        })
        .collect()
}

/// Compute size statistics for a batch of events
pub fn compute_batch_size_stats(events: &[NostrEvent]) -> Vec<SizeStats> {
    Format::all()
        .iter()
        .map(|&format| {
            let data = serialize_batch(events, format);
            let raw_bytes = data.len();
            let gzip_bytes = gzip_size(&data);
            let zstd_bytes = zstd_size(&data);

            SizeStats {
                format,
                raw_bytes,
                gzip_bytes,
                zstd_bytes,
            }
        })
        .collect()
}

/// Aggregate size statistics across multiple events
#[derive(Debug, Clone)]
pub struct AggregateSizeStats {
    pub format: Format,
    pub count: usize,
    pub total_raw: usize,
    pub total_gzip: usize,
    pub total_zstd: usize,
    pub min_raw: usize,
    pub max_raw: usize,
    pub avg_raw: f64,
}

impl AggregateSizeStats {
    pub fn avg_gzip(&self) -> f64 {
        self.total_gzip as f64 / self.count as f64
    }

    pub fn avg_zstd(&self) -> f64 {
        self.total_zstd as f64 / self.count as f64
    }
}

/// Compute aggregate size statistics for multiple events
pub fn compute_aggregate_stats(events: &[NostrEvent]) -> Vec<AggregateSizeStats> {
    let mut stats_by_format: HashMap<Format, Vec<SizeStats>> = HashMap::new();

    for event in events {
        for stat in compute_size_stats(event) {
            stats_by_format.entry(stat.format).or_default().push(stat);
        }
    }

    stats_by_format
        .into_iter()
        .map(|(format, stats)| {
            let count = stats.len();
            let total_raw: usize = stats.iter().map(|s| s.raw_bytes).sum();
            let total_gzip: usize = stats.iter().map(|s| s.gzip_bytes).sum();
            let total_zstd: usize = stats.iter().map(|s| s.zstd_bytes).sum();
            let min_raw = stats.iter().map(|s| s.raw_bytes).min().unwrap_or(0);
            let max_raw = stats.iter().map(|s| s.raw_bytes).max().unwrap_or(0);
            let avg_raw = total_raw as f64 / count as f64;

            AggregateSizeStats {
                format,
                count,
                total_raw,
                total_gzip,
                total_zstd,
                min_raw,
                max_raw,
                avg_raw,
            }
        })
        .collect()
}

/// Event distribution analysis
#[derive(Debug, Clone)]
pub struct DistributionAnalysis {
    pub total_events: usize,
    pub by_kind: HashMap<u16, usize>,
    pub by_size: HashMap<SizeCategory, usize>,
    pub by_tags: HashMap<TagCategory, usize>,
    pub avg_content_len: f64,
    pub avg_tag_count: f64,
}

impl DistributionAnalysis {
    pub fn from_events(events: &[NostrEvent]) -> Self {
        let total_events = events.len();

        let mut by_kind: HashMap<u16, usize> = HashMap::new();
        let mut by_size: HashMap<SizeCategory, usize> = HashMap::new();
        let mut by_tags: HashMap<TagCategory, usize> = HashMap::new();
        let mut total_content_len = 0usize;
        let mut total_tag_count = 0usize;

        for event in events {
            *by_kind.entry(event.kind).or_insert(0) += 1;
            *by_size.entry(event.size_category()).or_insert(0) += 1;
            *by_tags.entry(event.tag_category()).or_insert(0) += 1;
            total_content_len += event.content.len();
            total_tag_count += event.tag_count();
        }

        let avg_content_len = if total_events > 0 {
            total_content_len as f64 / total_events as f64
        } else {
            0.0
        };

        let avg_tag_count = if total_events > 0 {
            total_tag_count as f64 / total_events as f64
        } else {
            0.0
        };

        Self {
            total_events,
            by_kind,
            by_size,
            by_tags,
            avg_content_len,
            avg_tag_count,
        }
    }

    pub fn top_kinds(&self, n: usize) -> Vec<(u16, usize)> {
        let mut kinds: Vec<_> = self.by_kind.iter().map(|(&k, &v)| (k, v)).collect();
        kinds.sort_by(|a, b| b.1.cmp(&a.1));
        kinds.truncate(n);
        kinds
    }
}

/// Generate a markdown report of size comparisons
pub fn generate_size_report(events: &[NostrEvent]) -> String {
    let mut report = String::new();

    report.push_str("# Size Comparison Report\n\n");

    // Distribution analysis
    let dist = DistributionAnalysis::from_events(events);
    report.push_str("## Dataset Summary\n\n");
    report.push_str(&format!("- Total events: {}\n", dist.total_events));
    report.push_str(&format!(
        "- Average content length: {:.1} bytes\n",
        dist.avg_content_len
    ));
    report.push_str(&format!(
        "- Average tag count: {:.1}\n\n",
        dist.avg_tag_count
    ));

    // Top kinds
    report.push_str("### Top Event Kinds\n\n");
    report.push_str("| Kind | Count | Percentage |\n");
    report.push_str("|------|-------|------------|\n");
    for (kind, count) in dist.top_kinds(10) {
        let pct = 100.0 * count as f64 / dist.total_events as f64;
        report.push_str(&format!("| {} | {} | {:.1}% |\n", kind, count, pct));
    }
    report.push('\n');

    // Aggregate stats
    let stats = compute_aggregate_stats(events);

    report.push_str("## Size Statistics (per event)\n\n");
    report.push_str("| Format | Avg Raw | Avg Gzip | Avg Zstd | Min | Max |\n");
    report.push_str("|--------|---------|----------|----------|-----|-----|\n");

    let mut sorted_stats: Vec<_> = stats.iter().collect();
    sorted_stats.sort_by(|a, b| a.avg_raw.partial_cmp(&b.avg_raw).unwrap());

    for stat in &sorted_stats {
        report.push_str(&format!(
            "| {} | {:.0} | {:.0} | {:.0} | {} | {} |\n",
            stat.format.name(),
            stat.avg_raw,
            stat.avg_gzip(),
            stat.avg_zstd(),
            stat.min_raw,
            stat.max_raw,
        ));
    }
    report.push('\n');

    // Relative to JSON
    if let Some(json_stat) = stats.iter().find(|s| s.format == Format::Json) {
        report.push_str("## Size Relative to JSON\n\n");
        report.push_str("| Format | Raw | Gzip | Zstd |\n");
        report.push_str("|--------|-----|------|------|\n");

        for stat in &sorted_stats {
            let raw_pct = 100.0 * stat.avg_raw / json_stat.avg_raw;
            let gzip_pct = 100.0 * stat.avg_gzip() / json_stat.avg_gzip();
            let zstd_pct = 100.0 * stat.avg_zstd() / json_stat.avg_zstd();

            report.push_str(&format!(
                "| {} | {:.1}% | {:.1}% | {:.1}% |\n",
                stat.format.name(),
                raw_pct,
                gzip_pct,
                zstd_pct,
            ));
        }
    }

    report
}

/// Compress data with gzip and return the size
fn gzip_size(data: &[u8]) -> usize {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap().len()
}

/// Compress data with zstd and return the size
fn zstd_size(data: &[u8]) -> usize {
    zstd::encode_all(data, 3).unwrap().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abc123".to_string()],
                vec!["e".to_string(), "def456".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_size_stats() {
        let event = sample_event();
        let stats = compute_size_stats(&event);

        assert_eq!(stats.len(), 10);

        // All formats should produce non-zero sizes
        for stat in &stats {
            assert!(stat.raw_bytes > 0);
            assert!(stat.gzip_bytes > 0);
            assert!(stat.zstd_bytes > 0);
        }
    }

    #[test]
    fn test_distribution_analysis() {
        let events: Vec<NostrEvent> = (0..10)
            .map(|i| NostrEvent {
                id: [i as u8; 32],
                pubkey: [0u8; 32],
                created_at: 1234567890,
                kind: i % 3,
                tags: vec![],
                content: format!("Event {}", i),
                sig: [0u8; 64],
            })
            .collect();

        let dist = DistributionAnalysis::from_events(&events);

        assert_eq!(dist.total_events, 10);
        assert_eq!(dist.by_kind.len(), 3);
    }
}
