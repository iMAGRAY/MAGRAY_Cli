# Pre-commit hook для Windows PowerShell

Write-Host "[PRE-COMMIT] Checking code quality..." -ForegroundColor Yellow

$errors = 0

# Проверка форматирования
Write-Host "  • Checking formatting..." -ForegroundColor Cyan
cargo fmt --all --check 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "    ❌ Code not formatted! Run: cargo fmt --all" -ForegroundColor Red
    $errors++
} else {
    Write-Host "    ✅ Format OK" -ForegroundColor Green
}

# Проверка clippy
Write-Host "  • Running clippy..." -ForegroundColor Cyan
$clippy = cargo clippy --workspace --all-targets -- -D warnings 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "    ❌ Clippy errors found!" -ForegroundColor Red
    $clippy | Select-Object -First 10
    $errors++
} else {
    Write-Host "    ✅ Clippy OK" -ForegroundColor Green
}

# Проверка тестов
Write-Host "  • Running tests..." -ForegroundColor Cyan
cargo test --workspace --quiet 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "    ❌ Tests failed! Fix tests before committing" -ForegroundColor Red
    $errors++
} else {
    Write-Host "    ✅ Tests OK" -ForegroundColor Green
}

# Проверка на unwrap() в коде
Write-Host "  • Checking for unwrap() usage..." -ForegroundColor Cyan
$unwraps = Get-ChildItem -Path crates -Recurse -Filter "*.rs" | Select-String -Pattern "\.unwrap\(\)" | Measure-Object | Select-Object -ExpandProperty Count
if ($unwraps -gt 500) {
    Write-Host "    ⚠️ Too many unwrap() calls: $unwraps (max: 500)" -ForegroundColor Yellow
    Write-Host "       Consider using proper error handling" -ForegroundColor Yellow
}

# Итоговый результат
if ($errors -gt 0) {
    Write-Host "[PRE-COMMIT] ❌ BLOCKED: Fix $errors issue(s) before committing!" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[PRE-COMMIT] ✅ All checks passed!" -ForegroundColor Green
    exit 0
}