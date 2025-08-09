use anyhow::Result;
use std::collections::HashMap;

use crate::types::Layer;

#[derive(Clone, Debug, Default)]
pub struct KeywordIndex {
    inverted_index: HashMap<String, Vec<(String, u32)>>, // token -> [(doc_id, tf)]
    doc_len: HashMap<String, u32>,                       // doc_id -> length
    id_to_layer: HashMap<String, Layer>,                 // doc_id -> layer
    total_docs: usize,
}

impl KeywordIndex {
    pub fn new() -> Self { Self::default() }

    pub fn add_document(&mut self, id: &str, text: &str, layer: Layer) -> Result<()> {
        let tokens = tokenize(text);
        if tokens.is_empty() { return Ok(()); }

        let mut tf_map: HashMap<String, u32> = HashMap::new();
        for tok in tokens.iter() { *tf_map.entry(tok.clone()).or_insert(0) += 1; }

        let id_string = id.to_string();
        let length = tokens.len() as u32;
        self.doc_len.insert(id_string.clone(), length);
        self.id_to_layer.insert(id_string.clone(), layer);
        self.total_docs += 1;

        for (token, tf) in tf_map.into_iter() {
            self.inverted_index.entry(token).or_default().push((id_string.clone(), tf));
        }
        Ok(())
    }

    pub fn search(&self, query: &str, k: usize, layer: Option<Layer>) -> Result<Vec<(String, f32)>> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || self.total_docs == 0 { return Ok(vec![]); }

        let avgdl: f32 = {
            let sum: u64 = self.doc_len.values().map(|&l| l as u64).sum();
            (sum as f32) / (self.doc_len.len().max(1) as f32)
        };
        let k1: f32 = 1.2;
        let b: f32 = 0.75;
        let mut scores: HashMap<String, f32> = HashMap::new();

        for token in query_tokens {
            if let Some(postings) = self.inverted_index.get(&token) {
                let df = postings.len() as f32;
                let n = self.total_docs as f32;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1e-6).ln();

                for (doc_id, tf) in postings.iter() {
                    if let Some(dl) = self.doc_len.get(doc_id) {
                        if let Some(l_filter) = layer {
                            if let Some(doc_layer) = self.id_to_layer.get(doc_id) {
                                if *doc_layer != l_filter { continue; }
                            }
                        }
                        let tf = *tf as f32;
                        let dl = *dl as f32;
                        let denom = tf + k1 * (1.0 - b + b * (dl / avgdl.max(1e-6)));
                        let score = idf * ((tf * (k1 + 1.0)) / denom);
                        *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    }
                }
            }
        }

        let mut scored: Vec<(String, f32)> = scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let mut norm = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() { norm.push(ch.to_ascii_lowercase()); }
        else { norm.push(' '); }
    }
    norm.split_whitespace().map(|s| s.to_string()).collect()
}