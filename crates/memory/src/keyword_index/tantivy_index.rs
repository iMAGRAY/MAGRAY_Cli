#![cfg(feature = "keyword-search")]

use anyhow::Result;
use std::path::PathBuf;
use tantivy::{doc, schema::*, Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::types::Layer;

#[derive(Clone)]
pub struct KeywordIndex {
    index: Index,
    reader: IndexReader,
    writer: parking_lot::Mutex<IndexWriter>,
    f_id: Field,
    f_text: Field,
    f_layer: Field,
}

impl KeywordIndex {
    pub fn new_in_dir(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir).ok();
        let mut schema_builder = Schema::builder();
        let f_id = schema_builder.add_text_field("id", TEXT | STORED);
        let f_text = schema_builder.add_text_field("text", TEXT);
        let f_layer = schema_builder.add_text_field("layer", STRING | STORED);
        let schema = schema_builder.build();
        let index = Index::create_in_dir(&dir, schema.clone())?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
        let writer = index.writer(50_000_000)?; // 50MB
        Ok(Self {
            index,
            reader,
            writer: parking_lot::Mutex::new(writer),
            f_id,
            f_text,
            f_layer,
        })
    }

    pub fn add_document(&self, id: &str, text: &str, layer: Layer) -> Result<()> {
        let layer_str = match layer { Layer::Interact => "interact", Layer::Insights => "insights", Layer::Assets => "assets" };
        let mut writer = self.writer.lock();
        writer.add_document(doc!(self.f_id=>id.to_string(), self.f_text=>text.to_string(), self.f_layer=>layer_str))?;
        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query: &str, k: usize) -> Result<Vec<(String, f32)>> {
        use tantivy::query::QueryParser;
        let searcher = self.reader.searcher();
        let schema = self.index.schema();
        let parser = QueryParser::for_index(&self.index, vec![self.f_text]);
        let q = parser.parse_query(query)?;
        let top_docs = searcher.search(&q, &tantivy::collector::TopDocs::with_limit(k))?;
        let mut results = Vec::with_capacity(top_docs.len());
        for (score, addr) in top_docs {
            let retrieved = searcher.doc(addr)?;
            let id = retrieved
                .get_first(self.f_id)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            results.push((id, score as f32));
        }
        Ok(results)
    }
}