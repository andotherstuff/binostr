//! Event sampling utilities
//!
//! Tools for creating representative samples of events for benchmarking.

use std::collections::HashMap;
use std::path::Path;

use rand::prelude::*;
use rand::seq::SliceRandom;

use crate::event::{NostrEvent, SizeCategory, TagCategory};
use crate::loader::LoadError;

/// Event kinds to exclude from benchmarks.
///
/// These are kinds that appeared in the dataset but are either:
/// - Not documented in any NIP
/// - Application-specific custom kinds
/// - Test/spam kinds
///
/// Excluding these ensures benchmarks focus on representative real-world events.
///
/// Note: Kind numbers and their purposes:
/// - 0-9999: Regular events (stored by relays)
/// - 10000-19999: Replaceable events (only latest stored)
/// - 20000-29999: Ephemeral events (not stored)
/// - 30000-39999: Addressable events (identified by kind+pubkey+d-tag)
pub const EXCLUDED_KINDS: &[u16] = &[
    443,   // Unknown - not in any NIP
    1000,  // Unknown - not in any NIP (regular range)
    1009,  // Unknown - not in any NIP (regular range)
    10174, // Unknown - not in any NIP (replaceable range)
    11998, // Unknown - not in any NIP (replaceable range)
    30166, // Unknown - not in any NIP (addressable range)
    31111, // Unknown - not in any NIP (addressable range)
    31234, // Unknown - not in any NIP (addressable range)
    31402, // Unknown - not in any NIP (addressable range)
    32222, // Unknown - not in any NIP (addressable range)
    38225, // Unknown - not in any NIP (addressable range)
    38383, // Unknown - not in any NIP (addressable range)
];

/// Event sampler for creating benchmark datasets
pub struct EventSampler {
    events: Vec<NostrEvent>,
    rng: StdRng,
}

impl EventSampler {
    /// Create a new sampler with the given events
    pub fn new(events: Vec<NostrEvent>) -> Self {
        Self {
            events,
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a new sampler with a specific seed for reproducibility
    pub fn with_seed(events: Vec<NostrEvent>, seed: u64) -> Self {
        Self {
            events,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Load events from a directory and create a sampler
    /// Automatically filters out excluded kinds (see EXCLUDED_KINDS)
    pub fn from_directory<P: AsRef<Path>>(dir: P, limit: usize) -> Result<Self, LoadError> {
        let events = crate::loader::load_limited_from_directory(dir, limit)?;
        let mut sampler = Self::new(events);
        sampler.filter_excluded_kinds();
        Ok(sampler)
    }

    /// Load the complete corpus in stable file order and use a fixed seed.
    ///
    /// No event kinds are removed: unknown and application-specific kinds are
    /// part of real relay traffic and excluding them would bias a wire-format
    /// comparison. Call `filter_kinds` explicitly for a scoped experiment.
    pub fn from_directory_seeded<P: AsRef<Path>>(dir: P, seed: u64) -> Result<Self, LoadError> {
        let events = crate::loader::load_from_directory(dir)?;
        Ok(Self::with_seed(events, seed))
    }

    /// Filter out excluded event kinds (non-standard or test events)
    pub fn filter_excluded_kinds(&mut self) {
        self.events.retain(|e| !EXCLUDED_KINDS.contains(&e.kind));
    }

    /// Filter out specific event kinds
    pub fn filter_kinds(&mut self, kinds_to_remove: &[u16]) {
        self.events.retain(|e| !kinds_to_remove.contains(&e.kind));
    }

    /// Get the total number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the sampler is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get all events
    pub fn all(&self) -> &[NostrEvent] {
        &self.events
    }

    /// Get a random sample of n events
    pub fn random_sample(&mut self, n: usize) -> Vec<&NostrEvent> {
        self.events
            .choose_multiple(&mut self.rng, n.min(self.events.len()))
            .collect()
    }

    /// Get events of a specific kind
    pub fn by_kind(&self, kind: u16) -> Vec<&NostrEvent> {
        self.events.iter().filter(|e| e.kind == kind).collect()
    }

    /// Get a sample of events of a specific kind
    pub fn sample_kind(&mut self, kind: u16, n: usize) -> Vec<&NostrEvent> {
        let kind_events: Vec<_> = self.events.iter().filter(|e| e.kind == kind).collect();
        kind_events
            .choose_multiple(&mut self.rng, n.min(kind_events.len()))
            .cloned()
            .collect()
    }

    /// Get events by size category
    pub fn by_size(&self, category: SizeCategory) -> Vec<&NostrEvent> {
        self.events
            .iter()
            .filter(|e| e.size_category() == category)
            .collect()
    }

    /// Get a sample of events by size category
    pub fn sample_size(&mut self, category: SizeCategory, n: usize) -> Vec<&NostrEvent> {
        let size_events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.size_category() == category)
            .collect();
        size_events
            .choose_multiple(&mut self.rng, n.min(size_events.len()))
            .cloned()
            .collect()
    }

    /// Get events by tag count category
    pub fn by_tags(&self, category: TagCategory) -> Vec<&NostrEvent> {
        self.events
            .iter()
            .filter(|e| e.tag_category() == category)
            .collect()
    }

    /// Get a sample of events by tag count category
    pub fn sample_tags(&mut self, category: TagCategory, n: usize) -> Vec<&NostrEvent> {
        let tag_events: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.tag_category() == category)
            .collect();
        tag_events
            .choose_multiple(&mut self.rng, n.min(tag_events.len()))
            .cloned()
            .collect()
    }

    /// Get distribution of event kinds
    pub fn kind_distribution(&self) -> HashMap<u16, usize> {
        let mut dist = HashMap::new();
        for event in &self.events {
            *dist.entry(event.kind).or_insert(0) += 1;
        }
        dist
    }

    /// Get distribution of size categories
    pub fn size_distribution(&self) -> HashMap<SizeCategory, usize> {
        let mut dist = HashMap::new();
        for event in &self.events {
            *dist.entry(event.size_category()).or_insert(0) += 1;
        }
        dist
    }

    /// Get distribution of tag categories
    pub fn tag_distribution(&self) -> HashMap<TagCategory, usize> {
        let mut dist = HashMap::new();
        for event in &self.events {
            *dist.entry(event.tag_category()).or_insert(0) += 1;
        }
        dist
    }

    /// Create a stratified sample ensuring representation from key kinds
    ///
    /// Prioritizes kinds: 0, 1, 3, 4, 7, 10002, 30023
    /// Plus fills remaining slots with random events
    pub fn stratified_sample(&mut self, total: usize) -> Vec<NostrEvent> {
        let priority_kinds = [0, 1, 3, 4, 7, 10002, 30023];
        let per_kind = (total / (priority_kinds.len() + 1)).max(1);

        let mut sample = Vec::with_capacity(total);

        // Sample from each priority kind
        for kind in priority_kinds {
            let kind_events: Vec<_> = self.events.iter().filter(|e| e.kind == kind).collect();
            let kind_sample: Vec<_> = kind_events
                .choose_multiple(&mut self.rng, per_kind.min(kind_events.len()))
                .map(|&e| e.clone())
                .collect();
            sample.extend(kind_sample);
        }

        // Fill remaining with random events
        let remaining = total.saturating_sub(sample.len());
        if remaining > 0 {
            let random_sample: Vec<_> = self
                .events
                .choose_multiple(&mut self.rng, remaining.min(self.events.len()))
                .cloned()
                .collect();
            sample.extend(random_sample);
        }

        // Shuffle the final sample
        sample.shuffle(&mut self.rng);

        sample
    }

    /// Create samples organized by benchmark category
    pub fn create_benchmark_sets(&mut self) -> BenchmarkSets {
        BenchmarkSets {
            // By kind
            kind_0_profile: self.sample_kind(0, 100).into_iter().cloned().collect(),
            kind_1_notes: self.sample_kind(1, 100).into_iter().cloned().collect(),
            kind_3_follows: self.sample_kind(3, 100).into_iter().cloned().collect(),
            kind_4_dms: self.sample_kind(4, 100).into_iter().cloned().collect(),
            kind_7_reactions: self.sample_kind(7, 100).into_iter().cloned().collect(),
            kind_10002_relays: self.sample_kind(10002, 100).into_iter().cloned().collect(),
            kind_30023_articles: self.sample_kind(30023, 100).into_iter().cloned().collect(),

            // By size
            tiny: self
                .sample_size(SizeCategory::Tiny, 100)
                .into_iter()
                .cloned()
                .collect(),
            small: self
                .sample_size(SizeCategory::Small, 100)
                .into_iter()
                .cloned()
                .collect(),
            medium: self
                .sample_size(SizeCategory::Medium, 100)
                .into_iter()
                .cloned()
                .collect(),
            large: self
                .sample_size(SizeCategory::Large, 100)
                .into_iter()
                .cloned()
                .collect(),
            huge: self
                .sample_size(SizeCategory::Huge, 100)
                .into_iter()
                .cloned()
                .collect(),

            // By tag count
            no_tags: self
                .sample_tags(TagCategory::None, 100)
                .into_iter()
                .cloned()
                .collect(),
            few_tags: self
                .sample_tags(TagCategory::Few, 100)
                .into_iter()
                .cloned()
                .collect(),
            moderate_tags: self
                .sample_tags(TagCategory::Moderate, 100)
                .into_iter()
                .cloned()
                .collect(),
            many_tags: self
                .sample_tags(TagCategory::Many, 100)
                .into_iter()
                .cloned()
                .collect(),
            massive_tags: self
                .sample_tags(TagCategory::Massive, 100)
                .into_iter()
                .cloned()
                .collect(),

            // Random
            random_1000: self.random_sample(1000).into_iter().cloned().collect(),
        }
    }
}

/// Pre-organized benchmark datasets
#[derive(Debug, Clone)]
pub struct BenchmarkSets {
    // By kind
    pub kind_0_profile: Vec<NostrEvent>,
    pub kind_1_notes: Vec<NostrEvent>,
    pub kind_3_follows: Vec<NostrEvent>,
    pub kind_4_dms: Vec<NostrEvent>,
    pub kind_7_reactions: Vec<NostrEvent>,
    pub kind_10002_relays: Vec<NostrEvent>,
    pub kind_30023_articles: Vec<NostrEvent>,

    // By size
    pub tiny: Vec<NostrEvent>,
    pub small: Vec<NostrEvent>,
    pub medium: Vec<NostrEvent>,
    pub large: Vec<NostrEvent>,
    pub huge: Vec<NostrEvent>,

    // By tag count
    pub no_tags: Vec<NostrEvent>,
    pub few_tags: Vec<NostrEvent>,
    pub moderate_tags: Vec<NostrEvent>,
    pub many_tags: Vec<NostrEvent>,
    pub massive_tags: Vec<NostrEvent>,

    // Random mix
    pub random_1000: Vec<NostrEvent>,
}

impl BenchmarkSets {
    /// Check if all sets have at least some events
    pub fn is_valid(&self) -> bool {
        !self.random_1000.is_empty()
    }

    /// Get summary statistics
    pub fn summary(&self) -> String {
        format!(
            "BenchmarkSets:\n\
             - Kind 0 (Profile): {}\n\
             - Kind 1 (Notes): {}\n\
             - Kind 3 (Follows): {}\n\
             - Kind 4 (DMs): {}\n\
             - Kind 7 (Reactions): {}\n\
             - Kind 10002 (Relays): {}\n\
             - Kind 30023 (Articles): {}\n\
             - Tiny: {}\n\
             - Small: {}\n\
             - Medium: {}\n\
             - Large: {}\n\
             - Huge: {}\n\
             - No tags: {}\n\
             - Few tags: {}\n\
             - Moderate tags: {}\n\
             - Many tags: {}\n\
             - Massive tags: {}\n\
             - Random: {}",
            self.kind_0_profile.len(),
            self.kind_1_notes.len(),
            self.kind_3_follows.len(),
            self.kind_4_dms.len(),
            self.kind_7_reactions.len(),
            self.kind_10002_relays.len(),
            self.kind_30023_articles.len(),
            self.tiny.len(),
            self.small.len(),
            self.medium.len(),
            self.large.len(),
            self.huge.len(),
            self.no_tags.len(),
            self.few_tags.len(),
            self.moderate_tags.len(),
            self.many_tags.len(),
            self.massive_tags.len(),
            self.random_1000.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_events() -> Vec<NostrEvent> {
        (0..100)
            .map(|i| NostrEvent {
                id: [i as u8; 32],
                pubkey: [0u8; 32],
                created_at: 1234567890 + i as i64,
                kind: (i % 10) as u16,
                tags: (0..(i % 5)).map(|j| vec![format!("tag{}", j)]).collect(),
                content: "x".repeat(i * 10),
                sig: [0u8; 64],
            })
            .collect()
    }

    #[test]
    fn test_random_sample() {
        let events = make_test_events();
        let mut sampler = EventSampler::with_seed(events, 42);

        let sample = sampler.random_sample(10);
        assert_eq!(sample.len(), 10);
    }

    #[test]
    fn seeded_samples_are_reproducible() {
        let events = make_test_events();
        let mut first = EventSampler::with_seed(events.clone(), 42);
        let mut second = EventSampler::with_seed(events, 42);

        let first_ids: Vec<_> = first
            .random_sample(20)
            .iter()
            .map(|event| event.id)
            .collect();
        let second_ids: Vec<_> = second
            .random_sample(20)
            .iter()
            .map(|event| event.id)
            .collect();

        assert_eq!(first_ids, second_ids);
    }

    #[test]
    fn test_by_kind() {
        let events = make_test_events();
        let sampler = EventSampler::new(events);

        let kind_1 = sampler.by_kind(1);
        assert!(kind_1.iter().all(|e| e.kind == 1));
    }

    #[test]
    fn test_kind_distribution() {
        let events = make_test_events();
        let sampler = EventSampler::new(events);

        let dist = sampler.kind_distribution();
        assert_eq!(dist.get(&0), Some(&10)); // 0, 10, 20, ..., 90
    }
}
