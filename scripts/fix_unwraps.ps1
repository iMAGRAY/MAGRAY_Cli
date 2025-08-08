# PowerShell скрипт для исправления .unwrap() в Rust коде
# Фокусируется на основном коде, исключая тесты и бенчмарки

param(
    [Parameter(Position=0)]
    [string]$Path = "crates",
    
    [switch]$Apply,
    [switch]$ShowDetails
)

$ErrorActionPreference = "Stop"

# Цвета для вывода
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    $input | Write-Output
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-Host "🔍 Analyzing .unwrap() calls in production code..." -ForegroundColor Cyan

# Находим все .rs файлы, исключая тесты и бенчмарки
$rsFiles = Get-ChildItem -Path $Path -Filter "*.rs" -Recurse | 
    Where-Object { 
        $_.FullName -notmatch "\\(tests|benches|examples|target)\\" -and
        $_.Name -notmatch "^test_" -and
        $_.Name -notmatch "_test\.rs$"
    }

Write-Host "Found $($rsFiles.Count) production Rust files" -ForegroundColor Green

$totalUnwraps = 0
$fileStats = @{}
$criticalFiles = @()

foreach ($file in $rsFiles) {
    $content = Get-Content $file.FullName -Raw
    $unwrapCount = ([regex]::Matches($content, '\.unwrap\(\)')).Count
    
    if ($unwrapCount -gt 0) {
        $fileStats[$file.FullName] = $unwrapCount
        $totalUnwraps += $unwrapCount
        
        # Файлы в src/ более критичны чем в других местах
        if ($file.FullName -match "\\src\\") {
            $criticalFiles += @{
                Path = $file.FullName
                Count = $unwrapCount
                RelativePath = $file.FullName.Replace($PWD, "").TrimStart("\")
            }
        }
    }
}

Write-Host "`n📊 Summary:" -ForegroundColor Yellow
Write-Host "  Total unwrap() in production code: $totalUnwraps" -ForegroundColor White
Write-Host "  Files with unwrap(): $($fileStats.Count)" -ForegroundColor White

if ($criticalFiles.Count -gt 0) {
    Write-Host "`n❗ Critical files (in src/):" -ForegroundColor Red
    $criticalFiles | Sort-Object -Property Count -Descending | Select-Object -First 10 | ForEach-Object {
        Write-Host "  $($_.RelativePath): $($_.Count) unwrap()" -ForegroundColor Magenta
    }
}

function Fix-UnwrapInFile {
    param($FilePath)
    
    $content = Get-Content $FilePath -Raw
    $lines = $content -split "`n"
    $modified = $false
    $newLines = @()
    
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        
        if ($line -match '\.unwrap\(\)') {
            # Определяем контекст
            $inResultFn = $false
            $inTest = $false
            
            # Проверяем предыдущие строки для контекста
            for ($j = [Math]::Max(0, $i - 20); $j -lt $i; $j++) {
                if ($lines[$j] -match '-> Result|-> anyhow::Result') {
                    $inResultFn = $true
                }
                if ($lines[$j] -match '#\[test\]|#\[cfg\(test\)\]') {
                    $inTest = $true
                    break
                }
            }
            
            if ($inTest) {
                # В тестах оставляем expect
                $newLine = $line -replace '\.unwrap\(\)', '.expect("test assertion")'
            }
            elseif ($inResultFn) {
                # В функциях с Result используем ?
                $newLine = $line -replace '\.unwrap\(\)', '?'
            }
            else {
                # Определяем тип операции по контексту
                if ($line -match 'env::') {
                    $newLine = $line -replace '\.unwrap\(\)', '.expect("Environment variable not set")'
                }
                elseif ($line -match 'lock\(\)') {
                    $newLine = $line -replace '\.unwrap\(\)', '.expect("Failed to acquire lock")'
                }
                elseif ($line -match 'parse') {
                    $newLine = $line -replace '\.unwrap\(\)', '.expect("Failed to parse value")'
                }
                elseif ($line -match 'File::|fs::') {
                    $newLine = $line -replace '\.unwrap\(\)', '.expect("File operation failed")'
                }
                else {
                    $newLine = $line -replace '\.unwrap\(\)', '.unwrap_or_default()'
                }
            }
            
            if ($ShowDetails) {
                Write-Host "  Line $($i+1):" -ForegroundColor Yellow
                Write-Host "    - $line" -ForegroundColor Red
                Write-Host "    + $newLine" -ForegroundColor Green
            }
            
            $newLines += $newLine
            $modified = $true
        }
        else {
            $newLines += $line
        }
    }
    
    if ($modified -and $Apply) {
        $newContent = $newLines -join "`n"
        Set-Content -Path $FilePath -Value $newContent -NoNewline
        Write-Host "✅ Fixed $FilePath" -ForegroundColor Green
    }
    
    return $modified
}

if ($Apply) {
    Write-Host "`n⚠️  APPLY MODE - Files will be modified!" -ForegroundColor Yellow
    $confirm = Read-Host "Continue? (y/n)"
    if ($confirm -ne 'y') {
        Write-Host "Aborted." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "`nApplying fixes..." -ForegroundColor Cyan
    $fixedCount = 0
    
    foreach ($file in $criticalFiles) {
        if (Fix-UnwrapInFile -FilePath $file.Path) {
            $fixedCount++
        }
    }
    
    Write-Host "`n✅ Fixed $fixedCount files" -ForegroundColor Green
}
else {
    Write-Host "`n💡 Run with -Apply flag to fix these issues" -ForegroundColor Cyan
    
    # Показываем примеры исправлений для первого критичного файла
    if ($criticalFiles.Count -gt 0) {
        $sampleFile = $criticalFiles | Sort-Object -Property Count -Descending | Select-Object -First 1
        Write-Host "`nSample fixes for $($sampleFile.RelativePath):" -ForegroundColor Yellow
        
        $content = Get-Content $sampleFile.Path
        $lineNum = 0
        $samplesShown = 0
        
        foreach ($line in $content) {
            $lineNum++
            if ($line -match '\.unwrap\(\)' -and $samplesShown -lt 3) {
                Write-Host "  Line ${lineNum}:" -ForegroundColor Cyan
                Write-Host "    - $($line.Trim())" -ForegroundColor Red
                
                # Простая эвристика для предложения исправления
                if ($line -match '-> Result|-> anyhow::Result') {
                    $suggested = $line -replace '\.unwrap\(\)', '?'
                }
                elseif ($line -match 'lock\(\)') {
                    $suggested = $line -replace '\.unwrap\(\)', '.expect("Failed to acquire lock")'
                }
                else {
                    $suggested = $line -replace '\.unwrap\(\)', '.unwrap_or_default()'
                }
                
                Write-Host "    + $($suggested.Trim())" -ForegroundColor Green
                $samplesShown++
            }
        }
    }
}

Write-Host "`n📝 Next steps:" -ForegroundColor Yellow
Write-Host "  1. Review the suggested changes" -ForegroundColor White
Write-Host "  2. Run: .\scripts\fix_unwraps.ps1 -Apply" -ForegroundColor White
Write-Host "  3. Run: cargo check" -ForegroundColor White
Write-Host "  4. Run: cargo test" -ForegroundColor White