# Claude Tensor Language v2.0 (CTL2)
## Ultra-Compact JSON Format for AI-Driven Development

**Design Goal:** Describe tasks/architecture quickly; 1 JSON-line = 1 complete idea.

---

## 🚀 CORE FORMAT

### Structure
```json
{"k":"<kind>","id":"<slug>","t":"<title>","p":<1-5>,"e":"<ISO8601>","d":["deps"],"r":"<result>","m":{"cur":<val>,"tgt":<val>,"u":"<unit>"},"f":["tags"]}
```

**No spaces, strict key order: k → id → t → p → e → d → r → m → f**

### Keys Reference
| Key | Name | Type | Description |
|-----|------|------|-------------|
| k | kind | T/A/B/F/M/S/R/P/D/C/E | Task/Architecture/Bug/Feature/Metric/Solution/Refactor/Performance/Documentation/Component/Epic |
| id | identifier | string ≤32 chars | Unique slug, no spaces |
| t | title | string ≤40 chars | Brief description |
| p | priority | 1-5 (optional) | 1=low, 5=critical |
| e | effort/eta | ISO8601 duration | P3D = 3 days, PT4H = 4 hours |
| d | dependencies | array of ids | ["auth","db"] |
| r | result | string ≤20 chars | Expected outcome code |
| m | metric | {cur,tgt,u} object | Current/target/unit |
| f | flags/tags | array of strings | ["critical","security"] |

**Any extra field:** `x_<name>` (e.g., `x_risk`, `x_owner`)

---

## 📋 KIND TYPES

| Kind | Meaning | Example |
|------|---------|---------|
| T | Task | `{"k":"T","id":"add_auth","t":"Add JWT authentication","p":4,"e":"P2D"}` |
| A | Architecture | `{"k":"A","id":"microservices","t":"Service architecture","d":["api","db"]}` |
| B | Bug | `{"k":"B","id":"mem_leak","t":"Fix memory leak in parser","p":5,"e":"PT4H"}` |
| F | Feature | `{"k":"F","id":"search","t":"Full-text search API","e":"P1W","r":"search_ready"}` |
| M | Metric | `{"k":"M","id":"latency","t":"API response time","m":{"cur":250,"tgt":100,"u":"ms"}}` |
| S | Solution | `{"k":"S","id":"cache_redis","t":"Add Redis caching","r":"10x_speedup"}` |
| R | Refactor | `{"k":"R","id":"clean_auth","t":"Refactor auth module","e":"P3D"}` |
| P | Performance | `{"k":"P","id":"opt_query","t":"Optimize DB queries","m":{"cur":5,"tgt":0.5,"u":"s"}}` |
| D | Documentation | `{"k":"D","id":"api_docs","t":"Update API documentation","e":"PT8H"}` |
| C | Component | `{"k":"C","id":"user_service","t":"User management service","f":["core"]}` |
| E | Epic | `{"k":"E","id":"v2_launch","t":"Version 2.0 release","d":["search","auth","ui"]}` |

---

## 🔗 RELATIONSHIPS

Expressed through `d` (dependencies) and extended fields:

```json
{"k":"T","id":"impl_search","t":"Implement search","d":["design_search","db_index"]}
{"k":"T","id":"design_search","t":"Design search API","r":"api_spec"}
{"k":"T","id":"db_index","t":"Create search indexes","r":"indexes_ready"}
```

**Graph visualization:** Dependencies form a DAG (Directed Acyclic Graph)

---

## 📊 METRICS TRACKING

### Simple Metric
```json
{"k":"M","id":"cpu","t":"CPU usage","m":{"cur":85,"tgt":50,"u":"%"}}
```

### Complex Metric with History
```json
{"k":"M","id":"throughput","t":"Requests per second","m":{"cur":1000,"tgt":5000,"u":"rps"},"x_history":[800,900,950,1000],"x_trend":"up"}
```

### Health Indicators (using flags)
```json
{"k":"M","id":"health","t":"System health","m":{"cur":3,"tgt":5,"u":"score"},"f":["warning","degraded"]}
```

---

## 💻 PRACTICAL EXAMPLES

### Sprint Planning
```json
{"k":"E","id":"sprint_42","t":"Sprint 42: Search feature","d":["search_design","search_impl","search_test"]}
{"k":"T","id":"search_design","t":"Design search architecture","p":5,"e":"P2D","r":"design_doc"}
{"k":"T","id":"search_impl","t":"Implement search backend","p":4,"e":"P5D","d":["search_design"]}
{"k":"T","id":"search_test","t":"Test search functionality","p":3,"e":"P2D","d":["search_impl"]}
```

### Bug Tracking
```json
{"k":"B","id":"crash_ios","t":"App crashes on iOS 17","p":5,"e":"PT4H","f":["critical","ios"]}
{"k":"B","id":"ui_glitch","t":"Button overlap on mobile","p":2,"e":"PT2H","f":["ui","mobile"]}
```

### Architecture Documentation
```json
{"k":"A","id":"api_gateway","t":"API Gateway service","f":["ingress","routing"]}
{"k":"A","id":"auth_service","t":"Authentication service","d":["api_gateway"],"f":["security"]}
{"k":"A","id":"user_db","t":"User database","d":["auth_service"],"f":["postgres","persistent"]}
```

### Performance Optimization
```json
{"k":"P","id":"db_opt","t":"Database query optimization","m":{"cur":500,"tgt":50,"u":"ms"},"e":"P3D"}
{"k":"S","id":"add_index","t":"Add composite indexes","r":"10x_faster","d":["db_opt"]}
{"k":"M","id":"query_time","t":"Average query time","m":{"cur":500,"tgt":50,"u":"ms"},"x_queries":["user_search","product_list"]}
```

---

## 🛠️ TOOLING INTEGRATION

### CLI Tool
```bash
# Add task
ctl add '{"k":"T","id":"fix_bug","t":"Fix login bug","p":5,"e":"PT2H"}'

# Query tasks
ctl query --kind T --priority 5
ctl query --depends-on auth_service

# Update metric
ctl metric update cpu --current 45

# Visualize dependencies
ctl graph --format mermaid
```

### Git Integration
```bash
# Commit with CTL reference
git commit -m "feat: implement search API

CTL: {"k":"T","id":"search_impl","r":"completed"}"

# Auto-parse and update task status
```

### IDE Integration
```typescript
// VSCode extension auto-completes CTL JSON
// @component: {"k":"F","id":"dark_mode","t":"Add dark mode toggle","e":"P1D"}
function toggleDarkMode() {
  // Implementation
}

// Shows inline: ⏱️ 1 day | 🎯 Feature | 🔗 No deps
```

### Rust Code Annotations
```rust
// @component: {"k":"C","id":"vector_store","t":"Vector storage","m":{"cur":65,"tgt":100,"u":"%"}}
pub struct VectorStore {
    // implementation
}
```

### Auto-sync with docs-daemon
```bash
# Build once
cd docs-daemon && cargo build --release && cd ..

# Run sync
./docs-daemon/target/release/ctl-sync        # one-time
./docs-daemon/target/release/ctl-sync watch  # continuous
```

---

## 📐 FORMAT RULES

1. **One line = One concept** - Each JSON object is a complete, atomic unit
2. **No nested objects** - Except for `m` (metric) which has fixed structure
3. **Strict key order** - Always: k, id, t, then alphabetical for others
4. **Omit empty fields** - Don't include keys with null/empty values
5. **ISO 8601 durations** - P3D (3 days), PT4H (4 hours), P1W (1 week)
6. **ID format** - Lowercase, underscore_separated, ≤32 chars
7. **Title brevity** - ≤40 chars, action-oriented
8. **Result codes** - Short, meaningful outcomes (api_ready, bug_fixed)

---

## 🔄 MIGRATION FROM CTL v1

### Old Format (CTL v1)
```
#T001 implement_auth [high,3d,crypto_lib] → secure_login
```

### New Format (CTL v2)
```json
{"k":"T","id":"implement_auth","t":"Implement authentication","p":4,"e":"P3D","d":["crypto_lib"],"r":"secure_login"}
```

### Conversion Rules
- `#T001` → `{"k":"T","id":"t001"}`
- `[high,3d,deps]` → `"p":4,"e":"P3D","d":["deps"]`
- `→ result` → `"r":"result"`
- Priority: low=1, med=2, high=4, critical=5

---

## 🚀 ADVANTAGES

1. **Machine Readable** - Pure JSON, no parsing needed
2. **Git Friendly** - One line per change, clean diffs
3. **Grep-able** - Easy to search: `grep '"k":"T"' tasks.ctl`
4. **Streamable** - Process line by line, no loading entire file
5. **Extensible** - Add `x_*` fields without breaking compatibility
6. **Type Safe** - JSON schema validation available
7. **Language Agnostic** - Any language can read/write JSON

---

## 📱 REAL-WORLD USAGE

### Project File Structure
```
project/
├── .ctl/
│   ├── tasks.jsonl      # Active tasks
│   ├── completed.jsonl  # Completed items
│   ├── metrics.jsonl    # Current metrics
│   └── architecture.jsonl # System design
├── src/
└── README.md
```

### Daily Workflow
```bash
# Morning: Check critical tasks
ctl today
> {"k":"T","id":"fix_prod_bug","t":"Fix production crash","p":5,"e":"PT2H"}
> {"k":"T","id":"review_pr","t":"Review security PR","p":4,"e":"PT1H"}

# Track progress
ctl complete fix_prod_bug
ctl metric update error_rate --current 0

# End of day: Generate report
ctl report --format markdown
```

### CI/CD Integration
```yaml
# .github/workflows/ctl.yml
- name: Update CTL metrics
  run: |
    echo '{"k":"M","id":"build_time","t":"CI build duration","m":{"cur":'$BUILD_TIME',"tgt":300,"u":"s"}}' >> .ctl/metrics.jsonl
    echo '{"k":"M","id":"test_coverage","t":"Code coverage","m":{"cur":'$COVERAGE',"tgt":80,"u":"%"}}' >> .ctl/metrics.jsonl
```

---

## 🎯 SUCCESS METRICS

Track CTL adoption and effectiveness:

```json
{"k":"M","id":"ctl_adoption","t":"Team CTL usage","m":{"cur":0,"tgt":100,"u":"%"}}
{"k":"M","id":"task_clarity","t":"Task definition score","m":{"cur":0,"tgt":5,"u":"score"}}
{"k":"M","id":"delivery_predictability","t":"On-time delivery","m":{"cur":0,"tgt":95,"u":"%"}}
```

---

## 🌟 SUMMARY

CTL v2.0 transforms project management into data streams:
- **Write**: 1 line of JSON
- **Read**: Parse instantly  
- **Query**: grep/jq/SQL
- **Visualize**: Auto-generate graphs
- **Integrate**: Any tool that reads JSON

**The entire project state in machine-readable, human-writable format.**

```json
{"k":"M","id":"ctl_v2","t":"CTL v2.0 ready","m":{"cur":100,"tgt":100,"u":"%"},"f":["released","stable"]}
```