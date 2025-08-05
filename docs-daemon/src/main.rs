use notify::{Watcher, RecursiveMode, Result as NotifyResult, RecommendedWatcher, Event};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;
use walkdir::WalkDir;
use jsonschema::{Draft, JSONSchema};

#[derive(Debug, Serialize, Deserialize)]
struct FileCache {
    hashes: HashMap<String, String>,
}

#[derive(Debug)]
struct CtlSync {
    cache_path: PathBuf,
    crates_path: PathBuf,
    claude_path: PathBuf,
    cache: FileCache,
    component_regex: Regex,
    schema: JSONSchema,
}

impl CtlSync {
    fn new() -> Self {
        let base_path = std::env::current_dir().unwrap();
        let cache_path = base_path.join("cache.json");
        
        let mut cache = FileCache { hashes: HashMap::new() };
        if cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                if let Ok(loaded) = serde_json::from_str(&content) {
                    cache = loaded;
                }
            }
        }

        // Если запущен из docs-daemon, поднимаемся на уровень выше
        let crates_path = if base_path.ends_with("docs-daemon") {
            base_path.parent().unwrap().join("crates")
        } else {
            base_path.join("crates")
        };
        
        let claude_path = if base_path.ends_with("docs-daemon") {
            base_path.parent().unwrap().join("CLAUDE.md")
        } else {
            base_path.join("CLAUDE.md")
        };

        // CTL v2.0 JSON Schema
        let schema_json = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["k", "id", "t"],
            "properties": {
                "k": {
                    "type": "string",
                    "enum": ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E"]
                },
                "id": {
                    "type": "string",
                    "pattern": "^[a-z0-9_]{1,32}$"
                },
                "t": {
                    "type": "string",
                    "maxLength": 40
                },
                "p": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 5
                },
                "e": {
                    "type": "string",
                    "pattern": "^P(\\d+D)?(T(\\d+H)?(\\d+M)?)?$|^PT\\d+[HMS]$"
                },
                "d": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 10
                },
                "r": {
                    "type": "string",
                    "maxLength": 20
                },
                "m": {
                    "type": "object",
                    "required": ["cur", "tgt", "u"],
                    "properties": {
                        "cur": {"type": "number"},
                        "tgt": {"type": "number"},
                        "u": {"type": "string", "maxLength": 10}
                    }
                },
                "f": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 10
                }
            },
            "additionalProperties": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "number"},
                    {"type": "boolean"},
                    {"type": "array"},
                    {"type": "object"}
                ]
            }
        });
        
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_json)
            .expect("Invalid CTL schema");

        Self {
            cache_path,
            crates_path,
            claude_path,
            cache,
            component_regex: Regex::new(r#"//\s*@component:\s*(\{.*\})"#).unwrap(),
            schema,
        }
    }

    fn hash_file(&self, path: &Path) -> String {
        let content = fs::read(path).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&content);
        format!("{:x}", hasher.finalize())
    }

    fn extract_components(&self, content: &str, file_path: &str) -> Vec<Value> {
        let mut components = Vec::new();
        
        for (line_no, line) in content.lines().enumerate() {
            if let Some(caps) = self.component_regex.captures(line) {
                if let Some(json_str) = caps.get(1) {
                    println!("      Found annotation: {}", json_str.as_str());
                    match serde_json::from_str::<Value>(json_str.as_str()) {
                        Ok(mut component) => {
                            // Validate against CTL schema
                            if let Err(validation_errors) = self.schema.validate(&component) {
                                println!("        Schema validation failed:");
                                for error in validation_errors {
                                    println!("          - {}", error);
                                }
                                continue;
                            }
                            
                            // Add file location
                            if let Some(obj) = component.as_object_mut() {
                                obj.insert(
                                    "x_file".to_string(), 
                                    Value::String(format!("{}:{}", file_path, line_no + 1))
                                );
                            }
                            components.push(component);
                        }
                        Err(e) => {
                            println!("        JSON parse error: {}", e);
                        }
                    }
                }
            }
        }
        
        components
    }

    fn scan_crates(&mut self) -> (bool, Vec<Value>) {
        let mut all_components = Vec::new();
        let mut changed = false;
        let mut file_count = 0;

        // Scan for changes
        println!("Scanning directory: {:?}", self.crates_path);
        for entry in WalkDir::new(&self.crates_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            .filter(|e| !e.path().to_string_lossy().contains("target"))
        {
            file_count += 1;
            let path = entry.path();
            let relative_path = path.strip_prefix(&self.crates_path)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            
            let hash = self.hash_file(path);
            
            // Check if file changed
            if self.cache.hashes.get(&relative_path).map_or(true, |h| h != &hash) {
                println!("  Changed: {}", relative_path);
                self.cache.hashes.insert(relative_path.clone(), hash);
                changed = true;
            }
            
            // Always extract components from all files (not just changed ones)
            if let Ok(content) = fs::read_to_string(path) {
                let file_components = self.extract_components(&content, &relative_path);
                all_components.extend(file_components);
            }
        }

        // Sort components by kind and id
        all_components.sort_by(|a, b| {
            let a_kind = a.get("k").and_then(|v| v.as_str()).unwrap_or("");
            let b_kind = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
            let a_id = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let b_id = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
            
            a_kind.cmp(b_kind).then(a_id.cmp(b_id))
        });

        println!("Scanned {} files, {} changed", file_count, if changed { "some" } else { "none" });
        (changed, all_components)
    }

    fn update_claude(&self, components: &[Value]) {
        println!("Updating CLAUDE.md with {} components", components.len());
        if let Ok(mut content) = fs::read_to_string(&self.claude_path) {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            
            let mut new_section = format!(
                "# AUTO-GENERATED ARCHITECTURE\n\n\
                *Last updated: {}*\n\n\
                ## Components (CTL v2.0 Format)\n\n\
                ```json\n",
                timestamp
            );

            for component in components {
                new_section.push_str(&serde_json::to_string(component).unwrap());
                new_section.push('\n');
            }
            
            new_section.push_str("```");

            // Replace the auto-generated section
            // Find start of auto-generated section
            if let Some(start_pos) = content.find("# AUTO-GENERATED ARCHITECTURE") {
                // Find end position (next section or end of file)
                let end_pos = content[start_pos..]
                    .find("# AUTO-GENERATED COMPONENT STATUS")
                    .map(|pos| start_pos + pos)
                    .unwrap_or(content.len());
                
                // Replace the content
                let before = &content[..start_pos];
                let after = &content[end_pos..];
                content = format!("{}{}\n\n{}", before, new_section, after);
            } else {
                // If section doesn't exist, append it
                content.push_str("\n\n");
                content.push_str(&new_section);
            }
            
            fs::write(&self.claude_path, content).expect("Failed to write CLAUDE.md");
        }
    }

    fn save_cache(&self) {
        let json = serde_json::to_string_pretty(&self.cache).unwrap();
        fs::write(&self.cache_path, json).expect("Failed to save cache");
    }

    fn sync_once(&mut self) {
        println!("Scanning for CTL changes...");
        let (changed, components) = self.scan_crates();
        
        if changed {
            self.update_claude(&components);
            self.save_cache();
            println!("✅ Update complete");
        } else {
            println!("No changes detected");
        }
    }

    fn watch(&mut self) -> NotifyResult<()> {
        let (tx, rx) = channel();
        
        let mut watcher: RecommendedWatcher = Watcher::new(
            tx,
            notify::Config::default().with_poll_interval(Duration::from_secs(1))
        )?;
        
        watcher.watch(&self.crates_path, RecursiveMode::Recursive)?;
        
        println!("Watching for changes...");
        
        loop {
            match rx.recv() {
                Ok(Ok(Event { kind: _, paths, .. })) => {
                    if paths.iter().any(|p| p.extension().map_or(false, |ext| ext == "rs")) {
                        // Debounce
                        std::thread::sleep(Duration::from_millis(500));
                        
                        // Clear any pending events
                        while rx.try_recv().is_ok() {}
                        
                        self.sync_once();
                    }
                }
                Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
                Err(e) => eprintln!("Channel error: {:?}", e),
            }
        }
    }
}

fn main() {
    println!("CTL v2.0 Sync Daemon (Rust)");
    println!("===========================\n");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("once");
    
    let mut sync = CtlSync::new();
    
    match mode {
        "watch" => {
            sync.sync_once();
            if let Err(e) = sync.watch() {
                eprintln!("Watch error: {}", e);
            }
        }
        _ => {
            sync.sync_once();
        }
    }
}