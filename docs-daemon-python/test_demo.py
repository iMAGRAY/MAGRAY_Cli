#!/usr/bin/env python3
"""
CTL Sync Demo Script

Quick demonstration of the Python CTL sync daemon functionality.
"""

import tempfile
from pathlib import Path
from ctl_sync import CtlSync

def create_demo_files():
    """Create demo Rust files with CTL annotations"""
    
    # Create temporary directory structure
    temp_dir = Path(tempfile.mkdtemp())
    crates_dir = temp_dir / "crates" / "demo"
    crates_dir.mkdir(parents=True)
    
    # Demo file with CTL v2.0 annotations
    demo_v2_content = '''
// @component: {"k":"C","id":"memory_service","t":"Memory management service","m":{"cur":85,"tgt":95,"u":"%"},"f":["memory","hnsw"]}
pub struct MemoryService {
    store: VectorStore,
}

// @component: {"k":"S","id":"vector_store","t":"HNSW vector storage","m":{"cur":90,"tgt":100,"u":"%"},"d":["hnsw_lib"]}
impl VectorStore {
    pub fn new() -> Self {
        Self { /* ... */ }
    }
}
'''
    
    # Demo file with CTL v3.0 tensor annotations
    demo_v3_content = '''
// @ctl3: D[ai_pipeline:Service] := {grad[75->95] compose[gpu_engine,cpu_fallback] implies performance}
pub struct AIPipeline {
    gpu_engine: GpuEngine,
    cpu_fallback: CpuEngine,
}

// @ctl3: D[embedding_cache:Component] := {grad[95->100] parallel async partial optimization}
pub struct EmbeddingCache {
    lru: LruCache<String, Vec<f32>>,
}
'''
    
    # Write demo files
    (crates_dir / "memory_v2.rs").write_text(demo_v2_content)
    (crates_dir / "ai_v3.rs").write_text(demo_v3_content)
    
    # Create demo CLAUDE.md
    claude_content = '''# Demo CLAUDE.md

This is a demo CLAUDE.md file for testing CTL sync.

# AUTO-GENERATED ARCHITECTURE

*This section will be updated by CTL sync*

## Components (CTL v2.0/v3.0 Mixed Format)

```json
```
'''
    
    (temp_dir / "CLAUDE.md").write_text(claude_content)
    
    return temp_dir

def main():
    """Run demo"""
    print("CTL Sync Python Demo")
    print("=" * 50)
    
    # Create demo files
    print("Creating demo files...")
    demo_dir = create_demo_files()
    print(f"   Demo directory: {demo_dir}")
    
    # Initialize CTL sync
    print("\nInitializing CTL sync...")
    ctl_sync = CtlSync(demo_dir)
    
    # Perform sync
    print("\nRunning synchronization...")
    ctl_sync.sync_once()
    
    # Show results
    print("\nResults:")
    stats = ctl_sync.get_stats()
    print(f"   Total components found: {stats['total_components']}")
    print(f"   CTL v2.0 components: {stats['ctl2_components']}")
    print(f"   CTL v3.0 components: {stats['ctl3_components']}")
    
    # Show updated CLAUDE.md
    print("\nUpdated CLAUDE.md content:")
    print("-" * 30)
    claude_path = demo_dir / "CLAUDE.md"
    print(claude_path.read_text())
    
    print(f"\nDemo complete! Files created in: {demo_dir}")
    print("   You can inspect the generated files to see the results.")

if __name__ == '__main__':
    main()