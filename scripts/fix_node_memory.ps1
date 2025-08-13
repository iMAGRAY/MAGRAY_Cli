# Claude Code CLI Memory Fix Script
# Устраняет JavaScript heap out of memory errors

Write-Host "Claude Code CLI Memory Fix" -ForegroundColor Yellow
Write-Host "Diagnosing and fixing Node.js heap memory issues..." -ForegroundColor Cyan

# Проверка текущих переменных окружения
Write-Host "`nCurrent NODE_OPTIONS:" -ForegroundColor Green
$currentNodeOptions = $env:NODE_OPTIONS
if ($currentNodeOptions) {
    Write-Host "  $currentNodeOptions" -ForegroundColor White
} else {
    Write-Host "  Not set (using default ~4GB)" -ForegroundColor Yellow
}

# Детекция доступной системной памяти
Write-Host "`nSystem Memory Analysis:" -ForegroundColor Green
$totalMemoryGB = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1)
$availableMemoryGB = [math]::Round((Get-Counter '\Memory\Available MBytes').CounterSamples[0].CookedValue / 1024, 1)

Write-Host "  Total RAM: $totalMemoryGB GB" -ForegroundColor White
Write-Host "  Available: $availableMemoryGB GB" -ForegroundColor White

# Рекомендации по heap size
$recommendedHeapGB = [math]::Min([math]::Max($totalMemoryGB * 0.5, 8), 16)
$recommendedHeapMB = $recommendedHeapGB * 1024

Write-Host "`nRecommended Settings:" -ForegroundColor Green
Write-Host "  Heap Size: $recommendedHeapGB GB ($recommendedHeapMB MB)" -ForegroundColor White

# Применение оптимального NODE_OPTIONS
$optimizedNodeOptions = "--max-old-space-size=$recommendedHeapMB --expose-gc --trace-warnings"

Write-Host "`nApplying Memory Optimizations:" -ForegroundColor Green
Write-Host "  Setting NODE_OPTIONS: $optimizedNodeOptions" -ForegroundColor White

# Установка в текущей сессии
$env:NODE_OPTIONS = $optimizedNodeOptions

# Установка пользовательской переменной окружения (постоянно)
[Environment]::SetEnvironmentVariable("NODE_OPTIONS", $optimizedNodeOptions, "User")

Write-Host "`nMemory Settings Applied:" -ForegroundColor Green
Write-Host "  Session: Updated" -ForegroundColor White
Write-Host "  Permanent: Updated (restart terminal)" -ForegroundColor White

# Создание emergency override для критических случаев  
Write-Host "`nEmergency Mode Available:" -ForegroundColor Red
Write-Host "  For critical heap errors, run:" -ForegroundColor Yellow
Write-Host "  `$env:NODE_OPTIONS = '--max-old-space-size=16384 --expose-gc'" -ForegroundColor Cyan

# Создание cleanup команды
Write-Host "`nMemory Cleanup Commands:" -ForegroundColor Green
Write-Host "  Clear agent coordination: .\scripts\cleanup_agent_coordination.ps1" -ForegroundColor White
Write-Host "  Force GC in Node.js: global.gc() in REPL" -ForegroundColor White

Write-Host "`nRestart Required:" -ForegroundColor Yellow
Write-Host "  Restart terminal/IDE to apply permanent settings" -ForegroundColor White
Write-Host "  Or run: refreshenv (if using Chocolatey)" -ForegroundColor Cyan

Write-Host "`nNext Steps:" -ForegroundColor Blue
Write-Host "  1. Restart Claude Code CLI" -ForegroundColor White  
Write-Host "  2. Monitor memory usage" -ForegroundColor White
Write-Host "  3. Run cleanup scripts if needed" -ForegroundColor White

Write-Host "`nClaude Code CLI Memory Fix Complete!" -ForegroundColor Green