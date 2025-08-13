# Agent Coordination Cleanup Script
# Предотвращает накопление данных в agent-coordination.json

param(
    [int]$MaxEvents = 50,
    [int]$MaxFileLocks = 20,
    [switch]$DryRun
)

Write-Host "Agent Coordination Cleanup" -ForegroundColor Yellow
Write-Host "Preventing memory accumulation in coordination files..." -ForegroundColor Cyan

$coordFile = "C:\Users\1\.claude\agents\shared-journal\agent-coordination.json"

if (-not (Test-Path $coordFile)) {
    Write-Host "Coordination file not found: $coordFile" -ForegroundColor Yellow
    exit 0
}

# Загрузка координационного файла
try {
    $coordination = Get-Content $coordFile -Raw | ConvertFrom-Json
    Write-Host "Current Status:" -ForegroundColor Green
    Write-Host "  Tasks: $($coordination.todo_tasks.Count)" -ForegroundColor White
    Write-Host "  Active Agents: $($coordination.active_agents.Count)" -ForegroundColor White
    Write-Host "  File Locks: $($coordination.file_locks.Count)" -ForegroundColor White
    Write-Host "  Events: $($coordination.events.Count)" -ForegroundColor White
}
catch {
    Write-Host "Error reading coordination file: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

$needsCleanup = $false

# Проверка необходимости очистки событий
if ($coordination.events.Count -gt $MaxEvents) {
    Write-Host "Events cleanup needed: $($coordination.events.Count) > $MaxEvents" -ForegroundColor Yellow
    $needsCleanup = $true
}

# Проверка просроченных блокировок
$now = Get-Date
$expiredLocks = @()
foreach ($lock in $coordination.file_locks) {
    if ($lock.lease_expires_at) {
        $expiry = [DateTime]::Parse($lock.lease_expires_at)
        if ($expiry -lt $now) {
            $expiredLocks += $lock
        }
    }
}

if ($expiredLocks.Count -gt 0) {
    Write-Host "Expired locks found: $($expiredLocks.Count)" -ForegroundColor Yellow
    $needsCleanup = $true
}

if (-not $needsCleanup) {
    Write-Host "No cleanup needed" -ForegroundColor Green
    exit 0
}

if ($DryRun) {
    Write-Host "DRY RUN - Would perform:" -ForegroundColor Cyan
    if ($coordination.events.Count -gt $MaxEvents) {
        $toRemove = $coordination.events.Count - $MaxEvents
        Write-Host "  Remove $toRemove old events" -ForegroundColor White
    }
    if ($expiredLocks.Count -gt 0) {
        Write-Host "  Remove $($expiredLocks.Count) expired locks" -ForegroundColor White
    }
    exit 0
}

# Backup перед изменениями
$backupFile = "$coordFile.backup.$(Get-Date -Format 'yyyyMMdd_HHmmss')"
Copy-Item $coordFile $backupFile
Write-Host "Backup created: $backupFile" -ForegroundColor Green

# Очистка старых событий (оставить последние MaxEvents)
if ($coordination.events.Count -gt $MaxEvents) {
    $eventsToKeep = $coordination.events | Sort-Object ts -Descending | Select-Object -First $MaxEvents
    $coordination.events = $eventsToKeep | Sort-Object ts
    Write-Host "Cleaned events: kept $MaxEvents most recent" -ForegroundColor Green
}

# Удаление просроченных блокировок
if ($expiredLocks.Count -gt 0) {
    $validLocks = $coordination.file_locks | Where-Object { 
        $lock = $_
        -not ($expiredLocks | Where-Object { $_.path -eq $lock.path -and $_.owner -eq $lock.owner })
    }
    $coordination.file_locks = $validLocks
    Write-Host "Removed $($expiredLocks.Count) expired locks" -ForegroundColor Green
}

# Очистка завершенных задач старше 24 часов
$oneDayAgo = (Get-Date).AddDays(-1)
$activeTasks = @()
foreach ($task in $coordination.todo_tasks) {
    $keepTask = $true
    
    if ($task.status -eq "done" -or $task.status -eq "failed") {
        if ($task.lease_expires_at) {
            $taskTime = [DateTime]::Parse($task.lease_expires_at)
            if ($taskTime -lt $oneDayAgo) {
                $keepTask = $false
            }
        }
    }
    
    if ($keepTask) {
        $activeTasks += $task
    }
}

if ($activeTasks.Count -lt $coordination.todo_tasks.Count) {
    $cleanedTasks = $coordination.todo_tasks.Count - $activeTasks.Count
    $coordination.todo_tasks = $activeTasks
    Write-Host "Cleaned $cleanedTasks old completed tasks" -ForegroundColor Green
}

# Сохранение очищенного файла
try {
    $coordination | ConvertTo-Json -Depth 10 -Compress:$false | Set-Content $coordFile -Encoding UTF8
    Write-Host "Coordination file updated" -ForegroundColor Green
    
    # Добавление события о очистке
    $cleanupEvent = @{
        ts = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssZ")
        type = "coordination_cleanup"
        payload = @{
            events_cleaned = $($coordination.events.Count -lt $MaxEvents)
            locks_cleaned = $($expiredLocks.Count)
            backup_file = $backupFile
        }
    }
    
    # Обновление с событием очистки
    $coordination.events += $cleanupEvent
    $coordination | ConvertTo-Json -Depth 10 -Compress:$false | Set-Content $coordFile -Encoding UTF8
    
    Write-Host "Final Status:" -ForegroundColor Green
    Write-Host "  Tasks: $($coordination.todo_tasks.Count)" -ForegroundColor White
    Write-Host "  File Locks: $($coordination.file_locks.Count)" -ForegroundColor White  
    Write-Host "  Events: $($coordination.events.Count)" -ForegroundColor White
    Write-Host "Cleanup Complete!" -ForegroundColor Green
}
catch {
    Write-Host "Error saving coordination file: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Restoring from backup..." -ForegroundColor Yellow
    Copy-Item $backupFile $coordFile
    exit 1
}