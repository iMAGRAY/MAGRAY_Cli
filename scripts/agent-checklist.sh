#!/usr/bin/env bash
set -euo pipefail

echo "🤖 CLI-Agent Background Execution Checklist"
echo "============================================"

# Функция для проверки статуса
check_status() {
    if [ $? -eq 0 ]; then
        echo "✅ $1"
    else
        echo "❌ $1"
        return 1
    fi
}

# 1. Проверка критических блокеров
echo "🚨 1. Critical Blockers Status"
INCOMPLETE_FILE="implementation_plan/INCOMPLETE.md"
if [ -f "$INCOMPLETE_FILE" ]; then
    BLOCKERS=$(grep -c "БЛОКЕР" "$INCOMPLETE_FILE" 2>/dev/null || echo "0")
    echo "   📊 Active blockers: $BLOCKERS"
    
    if [ "$BLOCKERS" -gt 0 ]; then
        echo "   🔍 Current blockers:"
        grep -A 2 "БЛОКЕР" "$INCOMPLETE_FILE" | head -10
    fi
else
    echo "   ⚠️  INCOMPLETE.md not found"
fi

# 2. План и документация
echo ""
echo "📋 2. Planning & Documentation"
if [ -f "docs/workflow_state.md" ]; then
    LAST_UPDATE=$(stat -c %Y "docs/workflow_state.md" 2>/dev/null || echo "0")
    CURRENT_TIME=$(date +%s)
    AGE_HOURS=$(( (CURRENT_TIME - LAST_UPDATE) / 3600 ))
    
    if [ $AGE_HOURS -lt 24 ]; then
        echo "   ✅ Workflow state updated recently ($AGE_HOURS hours ago)"
    else
        echo "   ⚠️  Workflow state outdated ($AGE_HOURS hours ago)"
    fi
else
    echo "   ❌ workflow_state.md missing"
fi

# 3. Код и тесты
echo ""
echo "🧪 3. Code Quality & Tests"
if command -v cargo >/dev/null 2>&1; then
    echo "   🦀 Running Rust checks..."
    
    # Компиляция
    if cargo check --all --quiet 2>/dev/null; then
        echo "   ✅ Compilation successful"
    else
        echo "   ❌ Compilation failed"
    fi
    
    # Тесты
    if cargo test --all --quiet 2>/dev/null; then
        echo "   ✅ Tests passing"
    else
        echo "   ❌ Tests failing"
    fi
    
    # Проверка на заглушки
    STUBS=$(find . -name "*.rs" -exec grep -l "#INCOMPLETE\|fake_content\|заглушка" {} \; 2>/dev/null | wc -l)
    if [ "$STUBS" -eq 0 ]; then
        echo "   ✅ No stubs found in code"
    else
        echo "   ⚠️  $STUBS files contain stubs"
    fi
fi

# 4. Memory и индексация
echo ""
echo "🧠 4. Memory & Indexing"
MEMORY_DIR="$HOME/.ourcli"
if [ -d "$MEMORY_DIR" ]; then
    DB_SIZE=$(du -sh "$MEMORY_DIR" 2>/dev/null | cut -f1)
    echo "   📊 Memory store size: $DB_SIZE"
    
    # Проверка SQLite БД
    if [ -f "$MEMORY_DIR/projects/*/memory.db" ]; then
        echo "   ✅ Memory database exists"
    else
        echo "   ⚠️  Memory database not found"
    fi
else
    echo "   ❌ Memory directory not initialized"
fi

# 5. Todo и задачи
echo ""
echo "📝 5. Todo & Task Management"
if command -v cli >/dev/null 2>&1; then
    TODO_COUNT=$(cli todo list --state ready 2>/dev/null | wc -l || echo "0")
    echo "   📊 Ready tasks: $TODO_COUNT"
    
    BLOCKED_COUNT=$(cli todo list --state blocked 2>/dev/null | wc -l || echo "0")
    if [ "$BLOCKED_COUNT" -gt 0 ]; then
        echo "   ⚠️  Blocked tasks: $BLOCKED_COUNT"
    fi
else
    echo "   ⚠️  CLI tool not available"
fi

# 6. Производительность и метрики
echo ""
echo "📈 6. Performance & Metrics"
if [ -f "logs/app.log" ]; then
    LOG_SIZE=$(du -sh "logs/app.log" | cut -f1)
    ERROR_COUNT=$(grep -c "ERROR" "logs/app.log" 2>/dev/null || echo "0")
    
    echo "   📊 Log size: $LOG_SIZE"
    if [ "$ERROR_COUNT" -eq 0 ]; then
        echo "   ✅ No errors in logs"
    else
        echo "   ⚠️  $ERROR_COUNT errors in logs"
    fi
fi

# 7. Готовность к выполнению
echo ""
echo "🚀 7. Execution Readiness"
READY_SCORE=0

# Подсчёт готовности
[ "$BLOCKERS" -eq 0 ] && READY_SCORE=$((READY_SCORE + 2))
[ -f "docs/workflow_state.md" ] && READY_SCORE=$((READY_SCORE + 1))
[ "$STUBS" -eq 0 ] && READY_SCORE=$((READY_SCORE + 2))
[ -d "$MEMORY_DIR" ] && READY_SCORE=$((READY_SCORE + 1))

echo "   📊 Readiness Score: $READY_SCORE/6"

if [ "$READY_SCORE" -ge 5 ]; then
    echo "   ✅ System ready for autonomous execution"
elif [ "$READY_SCORE" -ge 3 ]; then
    echo "   ⚠️  System partially ready - some issues need attention"
else
    echo "   ❌ System not ready - critical issues must be resolved"
fi

echo ""
echo "🎯 Next Actions:"
if [ "$BLOCKERS" -gt 0 ]; then
    echo "   1. Resolve critical blockers in toolsvc/llm"
fi
if [ "$STUBS" -gt 0 ]; then
    echo "   2. Replace stubs with real implementations"
fi
if [ ! -d "$MEMORY_DIR" ]; then
    echo "   3. Initialize memory system: cli init"
fi

echo ""
echo "✨ Agent checklist completed!"
