# TUI Component Wireframes and Technical Specifications

## Main TUI Layout Wireframes

### Full Size Terminal (120x30+)
```
┌──────────────── MAGRAY CLI ──────────────────┐ ← Header Bar (1 row)
│ Mode: Plan | Agents: ● ● ● ○ ○ | 14:32:15     │
├──────────────────┬───────────────────────────┤
│ Plan Viewer      │ Preview Panel             │ ← Main Content (24 rows)
│ (48 cols)        │ (71 cols)                 │
│                  │                           │
│ 📋 Workflow      │ 📄 Changes Preview        │
│ ├─ 🧠 Analysis   │                           │
│ │   ✅ Complete  │ src/main.rs               │
│ ├─ 📝 Planning   │ -  old_function()         │
│ │   🔄 In Prog.  │ +  new_function()         │
│ ├─ ⚡ Execute    │                           │
│ │   ⏳ Waiting   │ Summary:                  │
│ └─ 🎯 Review     │ • 2 files modified        │
│     ⏳ Pending   │ • 0 files created         │
│                  │ • Risk: LOW               │
│                  │                           │
│ Dependencies:    │ ⚠️  Breaking changes      │
│ Analysis → Plan  │ detected in API           │
│ Plan → Execute   │                           │
│ Execute → Review │                           │
├──────────────────┴───────────────────────────┤ 
│ Timeline (5 rows)                            │ ← Timeline Panel
│ 14:32:15 🧠 IntentAnalyzer: Started          │
│ 14:32:16 📝 Planner: Generated 4 steps       │
│ 14:32:17 🔧 file_scanner: 23 files found     │
│ 14:32:18 💬 LLM: 1,250 tokens used          │
│ 14:32:19 ⚠️  Warning: Large changes pending  │
├──────────────────────────────────────────────┤
│[✅ Accept] [❌ Reject] [👁️ Step] [🛑 Abort]  │ ← Action Bar (1 row)  
├──────────────────────────────────────────────┤
│Progress: ████████░░ 80% | Mem: 45MB | F1=Help│ ← Status Bar (1 row)
└──────────────────────────────────────────────┘
```

### Compact Terminal (80x24)
```
┌─────────── MAGRAY CLI ────────────┐ ← Header (1 row)
│Mode: Plan │ Agents: ●●●○○ │14:32  │
├───────────────────────────────────┤
│ 📋 Workflow Steps                 │ ← Single Column (18 rows)
│ ├─ 🧠 Analysis      ✅ Complete   │
│ ├─ 📝 Planning      🔄 Progress   │
│ ├─ ⚡ Execute       ⏳ Waiting    │
│ └─ 🎯 Review        ⏳ Pending    │
│                                   │
│ 📄 Preview: 2 files modified      │
│ src/main.rs                       │
│ - old_function()                  │
│ + new_function()                  │
│                                   │
│ ⚠️  Breaking changes detected      │
│                                   │
│ Recent Events:                    │
│ 32:19 ⚠️ Warning: Large changes   │
│ 32:18 💬 LLM: 1.2K tokens        │
│ 32:17 🔧 Scanned 23 files        │
├───────────────────────────────────┤
│ [✅ Accept] [❌ Reject] [🛑 Abort] │ ← Actions (1 row)
├───────────────────────────────────┤
│ Progress: ████░ 80% | Mem: 45MB   │ ← Status (1 row)
└───────────────────────────────────┘
```

## Component Detailed Wireframes

### Plan Viewer Widget (Expanded)
```
┌─ Plan Viewer ─────────────────────────────────┐
│ 📋 Current Workflow: Code Refactoring         │
│                                               │
│ Workflow Status: 🔄 In Progress               │
│ Started: 14:30:15 | Elapsed: 2m 5s           │
│                                               │
│ Steps:                                        │
│ ├─ 🧠 Intent Analysis                         │
│ │   Status: ✅ Complete                      │
│ │   Result: Refactoring request identified   │
│ │   Duration: 15s                           │
│ │                                           │
│ ├─ 📝 Plan Generation                        │
│ │   Status: ✅ Complete                     │
│ │   Result: 4 steps planned                 │
│ │   Tools: file_scanner, code_transformer   │
│ │   Duration: 23s                          │
│ │                                          │
│ ├─ ⚡ Plan Execution                         │
│ │   Status: 🔄 In Progress (Step 2/4)      │
│ │   Current: Running code_transformer       │
│ │   Progress: ████████░░ 75%               │
│ │   ├─ ✅ Scan source files (12s)          │
│ │   ├─ 🔄 Apply transformations (in prog.) │
│ │   ├─ ⏳ Validate syntax                  │
│ │   └─ ⏳ Generate report                  │
│ │                                          │
│ └─ 🎯 Result Critique                       │
│     Status: ⏳ Waiting                      │
│     Will analyze execution results          │
│                                            │
│ Dependencies Satisfied: ✅                 │
│ Resource Usage: Normal                     │
│ Risk Level: 🟡 Medium                      │
└───────────────────────────────────────────┘

Navigation:
↑/↓  - Move between steps
→/←  - Expand/collapse details  
Enter - Show step details
Space - Select/unselect step
```

### Diff Viewer Widget (File Changes)
```
┌─ Changes Preview ──────────────────────────────┐
│ File: src/main.rs (Modified)                   │
│ Size: 2.3KB → 2.1KB (-200 bytes)              │
│ Risk: 🟡 Medium (API changes)                  │
│                                               │
│   1  use std::collections::HashMap;            │
│   2  use tokio::sync::RwLock;                 │
│   3                                           │
│ - 4  fn old_function() -> Result<(), Error> { │ ← Deletion
│ - 5      // Legacy implementation             │
│ - 6      deprecated_call()?;                  │
│ - 7      Ok(())                              │
│ - 8  }                                       │
│   9                                          │
│ +10  fn new_function() -> Result<(), Error> { │ ← Addition  
│ +11      // Improved implementation           │
│ +12      modern_call()?;                     │
│ +13      enhanced_logic();                   │
│ +14      Ok(())                              │
│ +15  }                                       │
│  16                                          │
│  17  fn main() {                             │
│ -18      old_function().unwrap();            │
│ +19      new_function().unwrap();            │
│  20  }                                       │
│                                             │
│ ──────────────────────────────────────────  │
│                                             │
│ Summary of Changes:                         │
│ • Functions renamed: 1                      │
│ • API breaking changes: 1                   │
│ • Security improvements: Yes                │
│ • Performance impact: Minimal               │
│                                             │
│ Files: [1/3] ◀ src/main.rs ▶               │
└─────────────────────────────────────────────┘

Controls:
PgUp/PgDn - Scroll diff
Tab       - Next/previous file  
f         - Toggle file list
/         - Search in diff
```

### Timeline Widget (Event Stream)
```
┌─ Workflow Timeline ──────────────────────────┐
│                                              │
│ 14:32:19 ⚠️  WARNING: Breaking API changes   │
│          detected in src/main.rs             │
│          Impact: Public interface modified   │
│                                              │
│ 14:32:18 💬 LLM Token Usage                  │
│          Provider: Anthropic Claude          │ 
│          Input: 2,150 tokens                 │
│          Output: 890 tokens                  │
│          Cost: $0.12                         │
│                                              │
│ 14:32:17 🔧 Tool Execution: code_transformer │
│          Status: Success                     │
│          Files processed: 3/3                │
│          Duration: 2.3s                      │
│          Memory: 12MB peak                   │
│                                              │
│ 14:32:16 📝 Plan Generated by Planner        │
│          Steps: 4 planned                    │
│          Estimated duration: 2-5 minutes     │
│          Risk assessment: Medium             │
│          Dependencies resolved: ✅           │
│                                              │
│ 14:32:15 🧠 IntentAnalyzer Started           │
│          Intent: Code refactoring request    │
│          Confidence: 94%                     │
│          Classification: DEVELOPMENT         │
│                                              │
│ Filter: [All] Warnings Errors Agents Tools   │
│ Auto-scroll: ✅ | Export: 📄                 │
└──────────────────────────────────────────────┘

Controls:
j/k      - Scroll timeline
Enter    - Expand event details
f        - Filter events
Ctrl+E   - Export timeline
```

### Action Buttons (Interactive States)
```
┌─ Actions ────────────────────────────────────┐
│                                              │
│ Plan Review Actions:                         │
│ ┌─────────────┐ ┌─────────────┐             │
│ │ ✅ Accept   │ │ ❌ Reject   │             │
│ │ Execute All │ │ Cancel Plan │             │
│ └─────────────┘ └─────────────┘             │
│                                              │
│ Advanced Actions:                            │  
│ ┌─────────────┐ ┌─────────────┐             │
│ │ 👁️ Step     │ │ 🛑 Abort    │             │
│ │ Through     │ │ Workflow    │             │  
│ └─────────────┘ └─────────────┘             │
│                                              │
│ Execution Controls:                          │
│ ┌─────────────┐ ┌─────────────┐             │
│ │ ⏸️ Pause     │ │ 🔄 Retry    │             │
│ │ Execution   │ │ Last Step   │             │
│ └─────────────┘ └─────────────┘             │
│                                              │
│ Current: [Accept Plan] selected              │
│ Hint: Tab to navigate, Enter to activate    │
└──────────────────────────────────────────────┘

Button States:
Normal:   [✅ Accept]
Hover:    [✅ Accept] (highlighted)
Pressed:  [✅ Accept] (inverted)
Disabled: [⚫ Accept] (dimmed)
```

## Loading and Error States

### Loading States
```
┌─ Plan Viewer ─────────────────────────────────┐
│ 📋 Workflow Status: Loading...                │
│                                               │
│ 🔄 Analyzing intent...                        │
│ ⠋ Processing user request                     │ ← Spinner
│                                               │
│ Progress: ████░░░░░░ 40%                      │
│ Estimated time remaining: 15 seconds          │
│                                               │
│ Steps to complete:                            │
│ • Validate input parameters                   │
│ • Select appropriate tools                    │
│ • Generate execution plan                     │
└───────────────────────────────────────────────┘
```

### Error States  
```
┌─ Plan Viewer ─────────────────────────────────┐
│ 📋 Workflow Status: ❌ Error                   │
│                                               │
│ ❌ Plan Generation Failed                      │
│                                               │
│ Error: Unable to connect to LLM provider      │
│ Details: Network timeout after 30s            │
│ Code: CONN_TIMEOUT_001                        │
│                                               │
│ Possible Solutions:                           │
│ • Check internet connection                   │
│ • Verify API credentials                      │
│ • Try a different LLM provider                │
│                                               │
│ Actions:                                      │
│ [🔄 Retry] [⚙️ Settings] [📞 Support]        │
└───────────────────────────────────────────────┘
```

### Empty States
```
┌─ Plan Viewer ─────────────────────────────────┐
│ 📋 No Active Workflow                         │
│                                               │
│          🌟                                   │
│                                               │
│ Ready to assist with your next task!          │
│                                               │
│ Start by running a command like:              │
│ • magray smart "help me with coding"         │
│ • magray tasks create "new feature"          │
│ • magray tools search git                    │
│                                               │
│ Or press 'h' for help and tutorials          │
└───────────────────────────────────────────────┘
```

## Keyboard Navigation Map

### Global Shortcuts
```
Navigation:
Tab          - Next panel/element
Shift+Tab    - Previous panel/element  
Enter        - Activate/select
Escape       - Cancel/back
Space        - Toggle/select

Workflow Control:
F1           - Help
F2           - Settings  
F5           - Refresh
F10          - Menu
Ctrl+C       - Emergency abort
Ctrl+L       - Clear/refresh
Ctrl+R       - Retry current action

Panel Shortcuts:
Ctrl+1       - Focus Plan Viewer
Ctrl+2       - Focus Preview Panel
Ctrl+3       - Focus Timeline  
Ctrl+4       - Focus Action Buttons

View Controls:
+/-          - Zoom in/out (font size)
Ctrl+←/→     - Resize panels horizontally
Ctrl+↑/↓     - Resize panels vertically
Ctrl+H       - Toggle high contrast
```

### Context-Specific Shortcuts
```
Plan Viewer:
↑/↓          - Navigate steps
→/←          - Expand/collapse
Enter        - Step details
d            - Show dependencies
r            - Show resources
t            - Show timing

Preview Panel:
PgUp/PgDn    - Scroll content
Home/End     - Top/bottom of file
Tab          - Next file
Shift+Tab    - Previous file
/            - Search in content
n/N          - Next/previous search result

Timeline:
j/k          - Scroll events (vim-style)
f            - Filter events
c            - Clear timeline
e            - Export timeline
x            - Expand all events
z            - Collapse all events
```

## Responsive Behavior Specifications

### Terminal Size Breakpoints
```rust
// Terminal Size Categories
enum TerminalSize {
    Minimal,  // < 80x24   - Single column, essential only
    Compact,  // 80x24     - Simplified layout, reduced details  
    Standard, // 100x30    - Standard layout, most features
    Large,    // 120x35+   - Full layout, all features
    XLarge,   // 150x40+   - Extended layout, extra panels
}

// Layout Adaptations
impl Layout {
    fn adapt_to_size(&mut self, size: TerminalSize) {
        match size {
            Minimal => {
                self.panels = 1;
                self.timeline_height = 3;
                self.details_level = Minimal;
            }
            Compact => {
                self.panels = 2; // Plan + Actions
                self.timeline_height = 4;
                self.details_level = Essential;
            }
            Standard => {
                self.panels = 3; // Plan + Preview + Timeline
                self.timeline_height = 5;
                self.details_level = Standard;
            }
            Large => {
                self.panels = 4; // All panels + extras
                self.timeline_height = 6;
                self.details_level = Detailed;
            }
        }
    }
}
```

### Content Prioritization
```
Priority 1 (Always Visible):
- Current workflow status
- Critical errors/warnings  
- Essential action buttons (Accept/Reject)
- Emergency abort capability

Priority 2 (Compact+ Terminals):
- Plan step details
- File change summaries
- Recent timeline events
- Progress indicators

Priority 3 (Standard+ Terminals):  
- Full diff preview
- Complete timeline
- Detailed step information
- Resource usage metrics

Priority 4 (Large+ Terminals):
- Extended context panels
- Advanced visualization
- Detailed agent health
- Historical data
```

---

This wireframe specification provides the complete visual and interaction design for implementing the TUI components, ensuring consistency across different terminal sizes and user scenarios.