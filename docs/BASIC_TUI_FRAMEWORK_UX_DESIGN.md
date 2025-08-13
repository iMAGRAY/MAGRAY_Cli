# Basic TUI Framework UX/UI Design Specification

## Problem Statement & User Context

**User Problem**: Полное отсутствие TUI блокирует Plan→Preview→Execute workflow - ключевую архитектурную цель проекта
**User Goals**: Интерактивное визуальное управление multi-agent workflow через минимальный но функциональный TUI
**Business Goals**: Разблокировать архитектурную цель "северной звезды UX" - Plan→Preview→Execute workflow
**Success Metrics**: MVP TUI поддерживает базовый workflow, интегрируется с AgentOrchestrator, обеспечивает CLI fallback

## User Research Insights

### Target Users
**Primary Persona**: Developer/Technical User
- **Goals**: Визуальный контроль над AI agent workflow, понимание что происходит, контроль над изменениями
- **Pain Points**: CLI черный ящик, непонятно что делает агент, нет контроля над изменениями
- **Context**: Работа в terminal, знаком с TUI приложениями (htop, vim), ценит минимализм

**Secondary Persona**: AI-Assisted Professional
- **Goals**: Безопасное использование AI для автоматизации задач
- **Pain Points**: Страх неконтролируемых изменений, нужен preview перед execution
- **Context**: Осторожный подход к AI automation, нужны понятные объяснения

### Current State Analysis
**Existing Flow Issues**: 
- CLI команды выполняются "в слепую" без предварительного просмотра
- Нет визуального представления multi-agent workflow
- Отсутствует интерактивное управление планами выполнения

**User Feedback**: Из архитектурного плана:
- "План → Предпросмотр → Выполнение: всегда виден план действий, дифф/симуляция изменений, один клик до выполнения"
- "Искренность и предсказуемость: детерминизм инструментов, прозрачные разрешения и последствия"

**Analytics Data**: 
- БЛОКЕР-4 статус: "Полное отсутствие TUI, Plan→Preview→Execute недоступен"
- Приоритет: MEDIUM (8-12 часов), но критичен для архитектурной целостности

## Information Architecture

### Site Map / TUI Structure
```
Main TUI Interface
├── Header Bar
│   ├── Application Title
│   ├── Current Mode (Plan/Preview/Execute)
│   └── Agent Status Indicators
├── Main Content Area
│   ├── Plan Viewer (Left Panel)
│   │   ├── Workflow Steps Tree
│   │   ├── Step Details
│   │   ├── Dependencies Visualization
│   │   └── Progress Indicators
│   ├── Preview Panel (Right Panel)
│   │   ├── File Diffs
│   │   ├── Command Previews
│   │   ├── Side Effects Summary
│   │   └── Risk Assessment
│   └── Timeline (Bottom Panel)
│       ├── Agent Events
│       ├── Tool Invocations
│       ├── LLM Token Usage
│       └── Error/Warning Log
├── Action Bar
│   ├── Accept Plan
│   ├── Reject Changes
│   ├── Step Through
│   ├── Abort Workflow
│   └── Show Help
└── Status Bar
    ├── Workflow Progress
    ├── Resource Usage
    ├── Last Update Time
    └── Keyboard Shortcuts Hint
```

### User Flow Diagram
```
[CLI Command] → [Intent Analysis] → [Plan Generation] → [Plan Display] → [User Review] → [Preview Generation] → [User Approval] → [Execution] → [Results Summary]
      ↓              ↓                  ↓                 ↓              ↓                 ↓                   ↓              ↓                ↓
  TUI Launch    Loading State      Plan Tree View    Interactive    Diff Display     Accept/Reject       Execution     Progress View    Status Update
                                                     Navigation      & Preview         Buttons            Monitoring
```

## Interaction Design

### Key User Journeys

1. **Primary Journey: Plan→Preview→Execute**
   - **Entry point**: CLI command triggers TUI launch
   - **Discovery**: Plan viewer shows workflow tree with step details
   - **Decision**: User reviews plan, expands/collapses steps, reads explanations
   - **Action**: User triggers preview generation, reviews diffs and side effects
   - **Success**: User accepts plan, monitors execution, sees results

2. **Secondary Journey: Emergency Abort**
   - **Mobile-specific considerations**: N/A (TUI is terminal-based)
   - **Accessibility considerations**: High contrast mode, keyboard-only navigation
   - **Error recovery paths**: ESC key abort, Ctrl+C emergency stop, graceful fallback to CLI

### State Management
- **Loading States**: 
  - Spinner animations for agent processing
  - Progress bars for long-running operations
  - Skeleton screens for plan tree loading
- **Empty States**: 
  - "No active workflow" when idle
  - "Plan generation failed" with retry option
  - "No changes detected" in preview mode
- **Error States**: 
  - Agent communication errors with retry buttons
  - Plan validation failures with explanations
  - Execution errors with rollback options
- **Success States**: 
  - Plan completed successfully
  - All changes applied
  - Ready for next command

## UI Component Specifications

### Layout & Grid System
- **Terminal Constraints**: Adaptive to terminal size (minimum 80x24, optimal 120x30+)
- **Panel System**: 
  - Left Panel: 40% width (Plan Viewer)
  - Right Panel: 40% width (Preview Panel) 
  - Bottom Panel: 20% height (Timeline)
  - Resizable with keyboard shortcuts (Ctrl+←/→/↑/↓)

### Typography Scale
```
# Terminal Text Rendering (fixed-width font)
Title: Bold + Underline           # Application header, panel titles
Heading: Bold                     # Step names, section headers  
Normal: Regular                   # Standard text, descriptions
Dimmed: Low intensity            # Secondary info, timestamps
Warning: Yellow + Bold           # Warnings, cautions
Error: Red + Bold               # Errors, failures
Success: Green + Bold           # Completed steps, success states
```

### Color System (Terminal Colors)
```
# Primary Colors (ANSI escape codes)
--primary: Blue (34m)           # Interactive elements, highlights
--secondary: Cyan (36m)         # Secondary highlights, links
--accent: Magenta (35m)         # Special states, current selection

# Semantic Colors  
--success: Green (32m)          # Completed, approved, safe
--warning: Yellow (33m)         # Caution, review needed
--error: Red (31m)              # Errors, dangerous actions
--info: Cyan (36m)              # Information, neutral

# Neutral Scale
--text-primary: White (37m)     # Primary text content
--text-secondary: Light Gray    # Secondary text, metadata
--text-dimmed: Dark Gray        # Disabled, less important
--background: Black (40m)       # Default background
--border: Gray                  # Panel borders, separators
```

### Component Library

#### Interactive Plan Viewer
```rust
// Plan Tree Widget
struct PlanTreeWidget {
    workflow: WorkflowState,
    expanded_nodes: HashSet<StepId>,
    selected_step: Option<StepId>,
    scroll_offset: usize,
}

// Visual representation:
┌─ Plan Viewer ─────────────────────────┐
│ 📋 Analysis Complete                   │
│ ├─ 🔍 Intent: Code Refactoring        │
│ ├─ 📝 Plan Generated (3 steps)        │
│ │  ├─ 📁 Scan source files            │
│ │  ├─ 🔄 Apply transformations        │
│ │  └─ ✅ Validate changes             │ 
│ └─ 🎯 Ready for preview               │
└───────────────────────────────────────┘

// States: collapsed, expanded, selected, completed, error, in_progress
// Interactions: arrow keys navigation, Enter to expand/collapse, Space to select
```

#### Diff Viewer Widget
```rust
// Diff Display Widget
struct DiffViewerWidget {
    file_diffs: Vec<FileDiff>,
    current_file: usize,
    scroll_position: usize,
    syntax_highlighting: bool,
}

// Visual representation:
┌─ Preview Panel ───────────────────────┐
│ 📄 src/main.rs                        │
│  -  fn old_function() {               │ 
│  -      // legacy implementation      │
│  -  }                                 │
│  +  fn new_function() {               │
│  +      // improved implementation    │ 
│  +  }                                 │
│                                       │
│ 🔧 2 files modified, 0 created        │
│ ⚠️  Warning: Breaking API changes     │
└───────────────────────────────────────┘

// Features: syntax highlighting, line numbers, folding, search
```

#### Action Buttons
```rust
// Action Button Widget
struct ActionButtonsWidget {
    buttons: Vec<ActionButton>,
    selected: usize,
    enabled: bool,
}

// Visual representation:
┌─ Actions ─────────────────────────────┐
│ [✅ Accept Plan]  [❌ Reject]         │
│ [👁️  Step Through] [🛑 Abort]        │ 
└───────────────────────────────────────┘

// States: enabled, disabled, pressed, focused
// Interactions: Tab/Shift+Tab to navigate, Enter to activate, mouse click
```

#### Timeline Widget  
```rust
// Timeline Widget
struct TimelineWidget {
    events: Vec<WorkflowEvent>,
    scroll_position: usize,
    filter: EventFilter,
    auto_scroll: bool,
}

// Visual representation:
┌─ Timeline ────────────────────────────┐
│ 14:32:15 🧠 IntentAnalyzer started    │
│ 14:32:16 📝 Plan generated (3 steps)  │
│ 14:32:17 🔧 Tool: file_scanner        │
│ 14:32:18 💬 LLM tokens: 1,250 used    │
│ 14:32:19 ⚠️  Warning: Large changes   │
└───────────────────────────────────────┘

// Features: auto-scroll, filtering, event details on selection
```

## Responsive Design Strategy

### Terminal Size Adaptation
```
# Minimum Size (80x24)
- Single column layout
- Simplified timeline
- Essential actions only
- Abbreviated text

# Optimal Size (120x30+)  
- Multi-panel layout
- Full timeline with details
- Complete action set
- Verbose explanations

# Large Terminal (150x40+)
- Additional side panels
- Expanded context views
- More detailed visualizations
- Advanced keyboard shortcuts
```

### Adaptive UI Elements
- **Plan Tree**: Collapses to single line items on narrow terminals
- **Diff Viewer**: Side-by-side vs unified diff based on width
- **Timeline**: Full timestamps vs relative times based on space
- **Action Buttons**: Full text vs icons based on available width

## Accessibility (Terminal Accessibility)

### Keyboard Navigation
- **Tab Order**: Plan tree → Preview panel → Action buttons → Timeline
- **Arrow Keys**: Tree navigation, diff scrolling, button selection
- **Function Keys**: F1=Help, F2=Rename, F3=Find, F10=Menu
- **Ctrl Combinations**: Ctrl+C=Abort, Ctrl+L=Refresh, Ctrl+R=Retry

### Screen Reader Support (Terminal-based)
```
# Semantic Terminal Output
- Use proper ANSI escape sequences for emphasis
- Provide text descriptions for symbols (📋 → "Plan:", 🔍 → "Analysis:")
- Clear hierarchy with indentation and spacing
- Status announcements for important state changes
```

### High Contrast Support
- **High Contrast Mode**: Toggle with Ctrl+H
- **Color Blind Support**: Use symbols + colors, not just colors
- **Terminal Settings**: Respect user's terminal color scheme
- **Brightness Adaptation**: Dimmed/bright modes for different environments

## Animation & Micro-interactions

### Motion Principles  
- **Duration**: 100-200ms for quick transitions (no blocking animations)
- **Easing**: Simple linear transitions (terminal limitations)
- **Purpose**: Indicate state changes, guide attention to updates

### Terminal Animations
```rust
// Spinner Animation
["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] // Rotating at 10 FPS

// Progress Bar Animation  
[                    ] 0%
[████                ] 20%
[████████████████████] 100%

// Typing Effect (for important messages)
"P" → "Pl" → "Pla" → "Plan" → "Plan ready"
```

### Interaction Feedback
- **Key Press**: Immediate visual response (highlight change)
- **Button Press**: Brief invert colors effect
- **Status Updates**: Smooth text updates with brief highlight
- **Error States**: Flash red briefly, then return to normal with error icon

## EventBus Integration (Real-time Updates)

### Event-Driven UI Updates
```rust
// TUI Event Handling
impl TuiApp {
    async fn handle_workflow_event(&mut self, event: WorkflowEvent) {
        match event.event_type {
            EventType::IntentAnalyzed => self.update_plan_viewer(&event),
            EventType::PlanGenerated => self.refresh_plan_tree(&event),
            EventType::ToolInvoked => self.add_timeline_event(&event),
            EventType::StepCompleted => self.update_progress(&event),
            EventType::WorkflowError => self.show_error_state(&event),
        }
        self.request_redraw();
    }
}

// Subscribed Event Topics
topics: [
    "workflow.intent_analyzed",
    "workflow.plan_generated", 
    "workflow.step_started",
    "workflow.step_completed",
    "workflow.tool_invoked",
    "workflow.error",
    "agent.health_changed"
]
```

### Real-time Data Sync
- **Workflow State**: Live updates from AgentOrchestrator
- **Progress Tracking**: Real-time step completion updates
- **Agent Health**: Live agent status indicators
- **Resource Usage**: Dynamic memory/CPU usage display

## CLI Fallback Strategy

### Fallback Triggers
```rust
// TUI Availability Detection
fn check_tui_support() -> bool {
    // Check for terminal capabilities
    std::env::var("TERM").is_ok() && 
    atty::is(atty::Stream::Stdout) &&
    terminal_size::terminal_size().is_some()
}

// Graceful Degradation
if !check_tui_support() {
    fallback_to_cli_mode();
}
```

### CLI Mode Features
- **Text-based Plan Display**: Structured text output instead of TUI
- **Confirmation Prompts**: Y/N prompts for approval steps
- **Progress Indicators**: Simple text-based progress (1/3, 2/3, 3/3)
- **Status Updates**: Regular text status updates instead of live updates

## Technical Implementation Architecture

### TUI Framework (ratatui)
```rust
// Main TUI Application Structure
pub struct TuiApp {
    // State management
    workflow_state: Arc<RwLock<WorkflowState>>,
    ui_state: UiState,
    
    // Event handling
    event_bus: EventBus<WorkflowEvent>,
    event_receiver: tokio::sync::mpsc::Receiver<WorkflowEvent>,
    
    // Widgets
    plan_viewer: PlanViewerWidget,
    diff_viewer: DiffViewerWidget,
    timeline: TimelineWidget,
    action_buttons: ActionButtonsWidget,
    
    // Configuration
    config: TuiConfig,
}
```

### Integration with AgentOrchestrator
```rust
// Orchestrator Integration
impl TuiApp {
    pub async fn connect_to_orchestrator(
        &mut self, 
        orchestrator: Arc<AgentOrchestrator>
    ) -> Result<(), TuiError> {
        // Subscribe to workflow events
        let event_stream = orchestrator.subscribe_to_events().await?;
        
        // Start event processing loop
        self.start_event_loop(event_stream).await?;
        
        Ok(())
    }
}
```

### Thread Safety & Performance
- **Async Event Handling**: Non-blocking UI updates
- **Efficient Rendering**: Only redraw changed areas
- **Memory Management**: Bounded event queues, cleanup old events
- **Resource Monitoring**: CPU/memory usage limits for UI thread

## Implementation Handoff

### Design Assets
- [x] Complete UI component specifications with visual representations
- [x] Color palette and symbol system for terminal rendering
- [x] Layout system for different terminal sizes
- [x] Event handling architecture for real-time updates

### Development Guidelines
1. **Rust/Ratatui Structure**: Use widget-based architecture, separate concerns
2. **Event-Driven Updates**: Never poll, always react to EventBus events
3. **Terminal Compatibility**: Test on different terminals (xterm, tmux, etc.)
4. **Error Recovery**: Always provide escape routes, graceful degradation
5. **Performance**: 60fps target for smooth interactions

### Quality Assurance Checklist
- [ ] Visual design matches specifications in different terminal sizes
- [ ] All keyboard shortcuts work correctly
- [ ] Real-time updates work without performance issues  
- [ ] CLI fallback activates correctly when TUI unavailable
- [ ] Error states display helpful information and recovery options
- [ ] Memory usage remains bounded during long workflows

## MVP Implementation Phases

### Phase 1: Basic TUI Foundation (4 hours)
```
БЛОКЕР-4.1: Create basic TUI framework
- TUI crate setup with ratatui dependency
- Basic layout with header, main area, status bar
- Event loop and keyboard handling
- Terminal initialization and cleanup
```

### Phase 2: Plan Viewer Implementation (4 hours) 
```
БЛОКЕР-4.2: Implement plan viewer
- Interactive workflow tree display
- Step details and dependencies
- Navigation with arrow keys
- Integration with WorkflowState from orchestrator
```

### Phase 3: Preview & Actions (4 hours)
```
БЛОКЕР-4.3: Add basic diff display  
- File diff viewer with syntax highlighting
- Accept/reject action buttons
- Basic Plan→Preview→Execute workflow
- CLI fallback implementation
```

### Total MVP Implementation: 12 hours
**Success Criteria**: Basic TUI supports Plan→Preview→Execute workflow with AgentOrchestrator integration

---

**Document Version**: 1.0  
**Last Updated**: 2025-08-13T14:00:00Z  
**Correlation ID**: BASIC_TUI_FRAMEWORK_DESIGN  
**Design Status**: Complete - Ready for Implementation  
**Implementation Estimate**: 12 hours (MVP), 20 hours (Full Featured)

This specification provides the complete UX/UI foundation for implementing the Basic TUI Framework that will unblock the critical Plan→Preview→Execute workflow in the MAGRAY CLI application.