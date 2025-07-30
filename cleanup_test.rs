use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    println!("=== CLEANUP OLD CACHE AND DB ===\n");
    
    // Get data directories
    let base_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("ourcli");
    
    let db_path = base_dir.join("lancedb");
    let cache_path = base_dir.join("cache").join("embeddings");
    
    println!("Checking paths:");
    println!("- DB: {}", db_path.display());
    println!("- Cache: {}", cache_path.display());
    
    // Check if they exist
    if db_path.exists() {
        println!("\n⚠️  Database exists - it may contain old mock embeddings");
        println!("   Size: {} MB", dir_size(&db_path)? as f64 / 1_048_576.0);
    }
    
    if cache_path.exists() {
        println!("\n⚠️  Cache exists - it may contain old mock embeddings");
        println!("   Size: {} MB", dir_size(&cache_path)? as f64 / 1_048_576.0);
    }
    
    println!("\nTo clean up old data and force fresh embeddings:");
    println!("1. Close all MAGRAY processes");
    println!("2. Delete these directories:");
    println!("   rmdir /s \"{}\"", db_path.display());
    println!("   rmdir /s \"{}\"", cache_path.display());
    println!("3. Run the test again");
    
    Ok(())
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        }
    }
    Ok(total)
}