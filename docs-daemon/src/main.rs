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
    ctl3_tensor_regex: Regex,
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
            ctl3_tensor_regex: Regex::new(r#"//\s*@ctl3:\s*(Ⱦ\[.*?\]\s*:=\s*\{[^}]*\})"#).unwrap(),
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
            // CTL v2.0 JSON format
            if let Some(caps) = self.component_regex.captures(line) {
                if let Some(json_str) = caps.get(1) {
                    println!("      Found CTL v2.0 annotation: {}", json_str.as_str());
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
            
            // CTL v3.0 Tensor format
            if let Some(caps) = self.ctl3_tensor_regex.captures(line) {
                if let Some(tensor_str) = caps.get(1) {
                    println!("      Found CTL v3.0 tensor: {}", tensor_str.as_str());
                    if let Some(component) = self.parse_ctl3_tensor(tensor_str.as_str(), file_path, line_no + 1) {
                        components.push(component);
                    }
                }
            }
        }
        
        components
    }

    fn parse_ctl3_tensor(&self, tensor_str: &str, file_path: &str, line_no: usize) -> Option<Value> {
        // CTL v3.0 tensor pattern: Ⱦ[id:type] := {tensor_operations}
        let tensor_regex = Regex::new(r"Ⱦ\[([^:]+):([^\]]+)\]\s*:=\s*\{([^}]*)\}").unwrap();
        
        if let Some(caps) = tensor_regex.captures(tensor_str) {
            let id = caps.get(1)?.as_str().trim();
            let type_str = caps.get(2)?.as_str().trim();
            let operations = caps.get(3)?.as_str().trim();
            
            // Convert CTL v3.0 to CTL v2.0 format for compatibility
            let mut component = serde_json::json!({
                "k": self.infer_kind_from_type(type_str),
                "id": id,
                "t": format!("{} (CTL v3.0)", type_str),
                "x_file": format!("{}:{}", file_path, line_no),
                "ctl3_tensor": tensor_str
            });
            
            // Parse tensor operations for metadata
            if let Some(maturity) = self.extract_maturity_tensor(operations) {
                component["m"] = maturity;
            }
            
            if let Some(flags) = self.extract_flags_tensor(operations) {
                component["f"] = serde_json::Value::Array(flags);
            }
            
            if let Some(dependencies) = self.extract_dependencies_tensor(operations) {
                component["d"] = serde_json::Value::Array(dependencies);
            }
            
            Some(component)
        } else {
            println!("        Failed to parse CTL v3.0 tensor: {}", tensor_str);
            None
        }
    }
    
    fn infer_kind_from_type(&self, type_str: &str) -> &str {
        match type_str.to_lowercase().as_str() {
            s if s.contains("test") => "T",
            s if s.contains("agent") => "A", 
            s if s.contains("batch") => "B",
            s if s.contains("function") => "F",
            s if s.contains("module") => "M",
            s if s.contains("service") => "S",
            s if s.contains("resource") => "R",
            s if s.contains("process") => "P",
            s if s.contains("data") => "D",
            s if s.contains("error") => "E",
            _ => "C" // Component by default
        }
    }
    
    fn extract_maturity_tensor(&self, operations: &str) -> Option<Value> {
        // Look for ∇[cur→tgt] pattern
        let maturity_regex = Regex::new(r"∇\[(\d+)→(\d+)\]").unwrap();
        if let Some(caps) = maturity_regex.captures(operations) {
            let cur: u32 = caps.get(1)?.as_str().parse().ok()?;
            let tgt: u32 = caps.get(2)?.as_str().parse().ok()?;
            
            Some(serde_json::json!({
                "cur": cur,
                "tgt": tgt,
                "u": "%"
            }))
        } else {
            None
        }
    }
    
    fn extract_flags_tensor(&self, operations: &str) -> Option<Vec<Value>> {
        // Look for feature flags in tensor operations
        let mut flags = Vec::new();
        
        // Extract from various tensor operators
        if operations.contains("⊗") { flags.push("tensor_composition".into()); }
        if operations.contains("⊕") { flags.push("tensor_addition".into()); }
        if operations.contains("∇") { flags.push("optimization".into()); }
        if operations.contains("∂") { flags.push("partial_implementation".into()); }
        if operations.contains("⟹") { flags.push("implication".into()); }
        if operations.contains("gpu") { flags.push("gpu".into()); }
        if operations.contains("ai") || operations.contains("ml") { flags.push("ai".into()); }
        if operations.contains("async") { flags.push("async".into()); }
        
        if flags.is_empty() { None } else { Some(flags) }
    }
    
    fn extract_dependencies_tensor(&self, operations: &str) -> Option<Vec<Value>> {
        // Look for ⊗[dep1, dep2] pattern
        let dep_regex = Regex::new(r"⊗\[([^\]]+)\]").unwrap();
        if let Some(caps) = dep_regex.captures(operations) {
            let deps_str = caps.get(1)?.as_str();
            let deps: Vec<Value> = deps_str
                .split(',')
                .map(|s| s.trim().to_string().into())
                .collect();
            Some(deps)
        } else {
            None
        }
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
                ## Components (CTL v2.0/v3.0 Mixed Format)\n\n\
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
    println!("CTL v3.0 Tensor Sync Daemon (Rust)");
    println!("===================================");
    println!("Supports:");
    println!("  - CTL v2.0 JSON format: // @component: {{...}}");
    println!("  - CTL v3.0 Tensor format: // @ctl3: Ⱦ[id:type] := {{...}}");
    println!("  - Tensor operators: ⊗,⊕,∇,∂,⟹ with auto-parsing");
    println!();

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