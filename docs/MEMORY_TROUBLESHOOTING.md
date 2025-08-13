# Claude Code CLI Memory Troubleshooting Guide

## 🚨 JavaScript Heap Out of Memory Error

### Problem Description
```
FATAL ERROR: Reached heap limit Allocation failed - JavaScript heap out of memory
```

This error occurs when Node.js process exceeds the default heap memory limit (~4GB on x64 systems).

## 🔧 Immediate Solutions

### 1. Increase Heap Limit (Quick Fix)
```powershell
# Increase to 8GB (recommended)
$env:NODE_OPTIONS = "--max-old-space-size=8192"

# Increase to 16GB (emergency)
$env:NODE_OPTIONS = "--max-old-space-size=16384"

# Make permanent (restart terminal after)
[Environment]::SetEnvironmentVariable("NODE_OPTIONS", "--max-old-space-size=8192", "User")
```

### 2. Automated Memory Fix
```powershell
# Run memory optimization script
.\scripts\fix_node_memory.ps1
```

### 3. Manual Cleanup
```powershell
# Clean agent coordination logs
.\scripts\cleanup_agent_coordination.ps1

# Dry run first
.\scripts\cleanup_agent_coordination.ps1 -DryRun
```

## 🔍 Root Causes & Prevention

### Common Memory Leak Sources

1. **Agent Coordination Logs**
   - `agent-coordination.json` grows without bounds
   - **Solution**: Automatic rotation when >200 lines

2. **EventBus Event Accumulation**
   - Events never cleaned from memory
   - **Solution**: TTL-based cleanup every 1 hour

3. **Large Context Objects**
   - Tool contexts and agent states accumulating
   - **Solution**: Implement context pooling

4. **File Content Caching**
   - Files cached indefinitely in memory
   - **Solution**: LRU cache with size limits

### Memory Monitoring Commands

```powershell
# Monitor Node.js memory usage
node --inspect --max-old-space-size=8192 your-app.js

# Force garbage collection (in Node.js REPL)
global.gc()

# Check current memory settings
echo $env:NODE_OPTIONS
```

## 🛠️ Long-term Optimizations

### 1. Agent Coordination Cleanup
- Automatically rotate logs when >200 lines
- Remove expired file locks
- Clean completed tasks older than 24h

### 2. EventBus Memory Management
```javascript
// Implement in EventBus
setInterval(() => {
    this.events = this.events.filter(event => 
        Date.now() - new Date(event.ts).getTime() < 3600000 // 1 hour
    );
}, 300000); // Every 5 minutes
```

### 3. Context Object Pooling
```javascript
// Implement context pooling
class ContextPool {
    constructor(maxSize = 100) {
        this.pool = [];
        this.maxSize = maxSize;
    }
    
    acquire() {
        return this.pool.pop() || this.createNew();
    }
    
    release(context) {
        if (this.pool.length < this.maxSize) {
            this.reset(context);
            this.pool.push(context);
        }
    }
}
```

### 4. Stream Processing
- Process large files as streams instead of loading into memory
- Use readable/writable streams for data transformation
- Implement backpressure handling

## 🚨 Emergency Procedures

### When CLI Completely Fails
1. **Kill all Node.js processes**:
   ```powershell
   Get-Process node* | Stop-Process -Force
   ```

2. **Clear all temporary data**:
   ```powershell
   Remove-Item "C:\Users\1\.claude\agents\shared-journal\*" -Force
   ```

3. **Restart with maximum memory**:
   ```powershell
   $env:NODE_OPTIONS = "--max-old-space-size=16384 --expose-gc"
   ```

### Memory Leak Detection
```powershell
# Enable memory profiling
$env:NODE_OPTIONS = "--max-old-space-size=8192 --inspect --expose-gc --trace-gc"

# Start CLI and monitor in Chrome DevTools
# Navigate to: chrome://inspect
```

## 📊 Memory Usage Guidelines

### Recommended Settings by System RAM

| System RAM | Heap Limit | NODE_OPTIONS |
|------------|------------|--------------|
| 8GB        | 4GB        | `--max-old-space-size=4096` |
| 16GB       | 8GB        | `--max-old-space-size=8192` |
| 32GB       | 16GB       | `--max-old-space-size=16384` |
| 64GB+      | 24GB       | `--max-old-space-size=24576` |

### Memory Budget Allocation
- **Agent Coordination**: 200MB max
- **EventBus Events**: 100MB max  
- **Tool Contexts**: 500MB max
- **File Cache**: 1GB max
- **Working Memory**: Remaining heap

## 🔄 Maintenance Schedule

### Daily (Automated)
- Clean expired file locks
- Rotate agent coordination logs
- Clear old EventBus events

### Weekly (Manual)
- Review memory usage patterns
- Check for memory leaks
- Update heap limits if needed

### Monthly (Analysis)
- Analyze memory growth trends
- Optimize high-usage components
- Update memory management strategies

## 📈 Performance Monitoring

### Key Metrics to Track
1. **Heap Usage**: Current vs. allocated
2. **GC Frequency**: Should be <10/minute under normal load
3. **Memory Growth Rate**: Should be <1MB/minute sustained
4. **Agent Count**: Active agents should be <20
5. **Event Queue Size**: Should be <1000 events

### Alerting Thresholds
- **Warning**: Heap usage >75%
- **Critical**: Heap usage >90%
- **Emergency**: GC frequency >20/minute

---

## ✅ Success Criteria

After implementing these fixes:
- [ ] Claude Code CLI starts without memory errors
- [ ] Memory usage stays below 75% of allocated heap
- [ ] Agent coordination logs rotate automatically
- [ ] EventBus events are cleaned regularly
- [ ] File caches respect size limits
- [ ] CLI remains responsive under normal load

---

*Memory Troubleshooting Guide v1.0 - 2025-08-13*  
*Addresses JavaScript heap out of memory in Claude Code CLI*