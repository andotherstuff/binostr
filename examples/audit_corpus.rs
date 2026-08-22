//! Audit the complete tracked corpus and write immutable-content metadata.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::Read;

use binostr::loader;
use binostr::validation;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
struct CorpusAudit {
    schema_version: u32,
    corpus_path: &'static str,
    compressed_bytes: u64,
    compressed_sha256: String,
    event_count: usize,
    unique_event_ids: usize,
    duplicate_event_ids: usize,
    valid_nip01_ids: usize,
    valid_bip340_signatures: usize,
    maximum_content_bytes: usize,
    maximum_tags: usize,
    maximum_values_in_one_tag: usize,
    kinds: BTreeMap<u16, usize>,
    provenance_status: &'static str,
}

fn file_digest(path: &str) -> (u64, String) {
    let mut file = File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    (length, hex::encode(digest.finalize()))
}

fn main() {
    const PATH: &str = "data/sample.pb.gz";
    let events = loader::load_from_directory("data").expect("tracked corpus is readable");
    let mut ids = HashSet::new();
    let mut valid_ids = 0;
    let mut valid_signatures = 0;
    let mut kinds = BTreeMap::new();
    let mut max_content = 0;
    let mut max_tags = 0;
    let mut max_values = 0;
    for event in &events {
        ids.insert(event.id);
        valid_ids += usize::from(validation::verify_id(event).is_ok());
        valid_signatures += usize::from(validation::verify_signature(event).is_ok());
        *kinds.entry(event.kind).or_default() += 1;
        max_content = max_content.max(event.content.len());
        max_tags = max_tags.max(event.tags.len());
        max_values = max_values.max(event.tags.iter().map(Vec::len).max().unwrap_or(0));
    }
    let (compressed_bytes, compressed_sha256) = file_digest(PATH);
    let audit = CorpusAudit {
        schema_version: 1,
        corpus_path: PATH,
        compressed_bytes,
        compressed_sha256,
        event_count: events.len(),
        unique_event_ids: ids.len(),
        duplicate_event_ids: events.len() - ids.len(),
        valid_nip01_ids: valid_ids,
        valid_bip340_signatures: valid_signatures,
        maximum_content_bytes: max_content,
        maximum_tags: max_tags,
        maximum_values_in_one_tag: max_values,
        kinds,
        provenance_status: "unknown: source relay, query, collection dates, transformations, and redistribution basis are not recorded",
    };
    fs::create_dir_all("results").unwrap();
    fs::write(
        "results/corpus-audit.json",
        serde_json::to_vec_pretty(&audit).unwrap(),
    )
    .unwrap();
    assert_eq!(
        valid_ids,
        events.len(),
        "corpus contains invalid NIP-01 IDs"
    );
    assert_eq!(
        valid_signatures,
        events.len(),
        "corpus contains invalid signatures"
    );
    println!(
        "Audited {} events; wrote results/corpus-audit.json",
        events.len()
    );
}
