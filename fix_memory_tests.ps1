# PowerShell скрипт для исправления тестов memory crate

$files = @(
    "crates/memory/tests/integration_full_workflow.rs",
    "crates/memory/tests/test_full_system.rs",
    "crates/memory/tests/test_metrics_integration.rs",
    "crates/memory/tests/test_promotion.rs",
    "crates/memory/tests/test_qwen3_complete.rs",
    "crates/memory/tests/test_qwen3_integration.rs"
)

foreach ($file in $files) {
    Write-Host "Fixing $file..."
    
    # Читаем файл
    $content = Get-Content $file -Raw
    
    # Заменяем паттерн MemoryConfig без ..Default::default()
    # Ищем закрывающую скобку структуры и добавляем перед ней
    $pattern = '(let\s+\w+\s*=\s*MemoryConfig\s*\{[^}]+)(\s*\};)'
    
    # Проверяем, есть ли уже ..Default::default()
    if ($content -notmatch '\.\.Default::default\(\)') {
        $content = $content -replace $pattern, '$1,`n        ..Default::default()$2'
    }
    
    # Сохраняем файл
    Set-Content -Path $file -Value $content -NoNewline
    
    Write-Host "Fixed $file"
}

Write-Host "All files fixed!"