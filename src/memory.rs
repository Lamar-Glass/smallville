use std::collections::HashSet;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MemoryKind {
    Observation,
    Dialogue,
    Reflection,
}

pub struct MemoryEntry {
    pub content: String,
    pub importance: f32,
    pub created_minutes: f64,
    pub kind: MemoryKind,
    pub embedding: Option<Vec<f32>>,
}

pub struct MemoryStream {
    pub entries: Vec<MemoryEntry>,
}

impl MemoryStream {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn add(
        &mut self,
        content: &str,
        importance: f32,
        now_minutes: f64,
        kind: MemoryKind,
        embedding: Option<Vec<f32>>,
    ) {
        self.entries.push(MemoryEntry {
            content: content.to_string(),
            importance,
            created_minutes: now_minutes,
            kind,
            embedding,
        });
    }

    pub fn recent(&self, n: usize) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .take(n)
            .map(|e| e.content.clone())
            .collect()
    }

    pub fn recent_annotated(&self, n: usize) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .take(n)
            .map(|e| {
                let tag = match e.kind {
                    MemoryKind::Observation => "📋",
                    MemoryKind::Dialogue => "💬",
                    MemoryKind::Reflection => "🔭",
                };
                format!("{tag} {}", e.content)
            })
            .collect()
    }

    pub fn retrieve(
        &self,
        query: &str,
        query_embed: Option<&Vec<f32>>,
        now_minutes: f64,
        k: usize,
    ) -> Vec<String> {
        let query_tokens = tokens(query);
        let mut scored: Vec<(f32, usize)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let hours_since = ((now_minutes - e.created_minutes).max(0.0)) / 60.0;
                let recency = 0.98f32.powf(hours_since as f32);
                let importance = (e.importance / 10.0).clamp(0.0, 1.0);
                let relevance = match (&e.embedding, query_embed) {
                    (Some(a), Some(b)) => cosine(a, b),
                    _ => jaccard(&query_tokens, &tokens(&e.content)),
                };
                (0.35 * recency + 0.35 * importance + 0.3 * relevance, i)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(k)
            .map(|(_, i)| self.entries[i].content.clone())
            .collect()
    }
}

pub fn tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && w.len() > 1)
        .map(|w| w.to_string())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_ranks_relevant_over_stale() {
        let mut m = MemoryStream::new();
        m.add("The café owner loves hosting parties", 8.0, 0.0, MemoryKind::Observation, None);
        m.add("I watered the marigolds at the park", 3.0, 0.0, MemoryKind::Observation, None);
        let top = m.retrieve("what do I know about the café", None, 100.0, 2);
        assert!(top[0].contains("café"));
    }

    #[test]
    fn retrieval_favors_recent_when_equally_relevant() {
        let mut m = MemoryStream::new();
        m.add("talking about music with Eddy", 5.0, 0.0, MemoryKind::Observation, None);
        m.add("talking about music with Eddy", 5.0, 500.0, MemoryKind::Observation, None);
        let top = m.retrieve("eddy music", None, 1000.0, 1);
        assert_eq!(top.len(), 1);
    }

    #[test]
    fn tokens_are_lowercase_words() {
        let t = tokens("Hello, Smallville! 2 words.");
        assert!(t.contains("hello"));
        assert!(t.contains("smallville"));
        assert!(!t.contains("2"));
    }
}
