# Check test coverage for MAGRAY project
Write-Host "Analyzing test coverage for MAGRAY..." -ForegroundColor Cyan

# Count source files and test files
$sourceFiles = @(Get-ChildItem -Path "crates" -Recurse -Filter "*.rs" | Where-Object { 
    $_.FullName -notmatch "\\tests\\" -and 
    $_.FullName -notmatch "\\examples\\" -and
    $_.FullName -notmatch "\\benches\\" -and
    $_.FullName -notmatch "\\target\\"
})

$testFiles = @(Get-ChildItem -Path "crates" -Recurse -Filter "*.rs" | Where-Object { 
    $_.FullName -match "\\tests\\" -or
    $_.Name -match "_test\.rs$" -or
    $_.Name -match "^test_"
})

$sourceCount = $sourceFiles.Count
$testCount = $testFiles.Count

Write-Host "`nFile statistics:" -ForegroundColor Yellow
Write-Host "   Source files: $sourceCount"
Write-Host "   Test files: $testCount"
Write-Host "   Ratio: $([math]::Round($testCount / $sourceCount * 100, 1))%"

# Count test functions
$testFunctions = 0
foreach ($file in $testFiles) {
    $content = Get-Content $file.FullName -Raw -ErrorAction SilentlyContinue
    if ($content) {
        $matches = [regex]::Matches($content, "#\[test\]|#\[tokio::test\]")
        $testFunctions = $testFunctions + $matches.Count
    }
}

# Count functions in source
$sourceFunctions = 0
foreach ($file in $sourceFiles) {
    $content = Get-Content $file.FullName -Raw -ErrorAction SilentlyContinue
    if ($content) {
        $pubFnMatches = [regex]::Matches($content, "pub\s+(async\s+)?fn\s+\w+")
        $sourceFunctions = $sourceFunctions + $pubFnMatches.Count
    }
}

Write-Host "`nFunction statistics:" -ForegroundColor Yellow
Write-Host "   Public functions: $sourceFunctions"
Write-Host "   Test functions: $testFunctions"
if ($sourceFunctions -gt 0) {
    Write-Host "   Function coverage: $([math]::Round($testFunctions / $sourceFunctions * 100, 1))%"
}

# Analyze by crate
Write-Host "`nCoverage by module:" -ForegroundColor Yellow
$crates = Get-ChildItem -Path "crates" -Directory

$totalSrcLines = 0
$totalTestLines = 0

foreach ($crate in $crates) {
    $crateSrc = @(Get-ChildItem -Path "$($crate.FullName)\src" -Filter "*.rs" -Recurse -ErrorAction SilentlyContinue)
    $crateTests = @(Get-ChildItem -Path "$($crate.FullName)\tests" -Filter "*.rs" -ErrorAction SilentlyContinue)
    
    if ($crateSrc.Count -gt 0) {
        $srcLines = 0
        foreach ($file in $crateSrc) {
            $lines = @(Get-Content $file.FullName -ErrorAction SilentlyContinue)
            $srcLines = $srcLines + $lines.Count
        }
        
        $testLines = 0
        if ($crateTests.Count -gt 0) {
            foreach ($file in $crateTests) {
                $lines = @(Get-Content $file.FullName -ErrorAction SilentlyContinue)
                $testLines = $testLines + $lines.Count
            }
        }
        
        # Check for tests in src files
        foreach ($file in $crateSrc) {
            $content = Get-Content $file.FullName -Raw -ErrorAction SilentlyContinue
            if ($content -and $content -match "#\[cfg\(test\)\]") {
                $testLines = $testLines + 100 # Approximate
            }
        }
        
        $totalSrcLines = $totalSrcLines + $srcLines
        $totalTestLines = $totalTestLines + $testLines
        
        $coverage = if ($srcLines -gt 0) { [math]::Round($testLines / $srcLines * 100, 1) } else { 0 }
        $status = if ($coverage -ge 70) { "[OK]" } elseif ($coverage -ge 40) { "[WARN]" } else { "[LOW]" }
        
        Write-Host "   $status $($crate.Name): $coverage% (src: $srcLines lines, test: $testLines lines)"
    }
}

# Estimate total coverage
$totalCoverage = if ($totalSrcLines -gt 0) { [math]::Round($totalTestLines / $totalSrcLines * 100, 1) } else { 0 }

Write-Host "`nTotal test coverage:" -ForegroundColor Green
Write-Host "   Estimated: ~$totalCoverage%"
Write-Host "   Target: 80%"
if ($totalCoverage -ge 80) {
    Write-Host "   Status: ACHIEVED" -ForegroundColor Green
} else {
    $needed = [math]::Round(80 - $totalCoverage, 1)
    Write-Host "   Status: Need $needed% more" -ForegroundColor Yellow
}

# Run actual tests to get real numbers
Write-Host "`nRunning tests to get actual numbers..." -ForegroundColor Cyan
$testOutput = cargo test --workspace --lib --quiet 2>&1 | Out-String
$passedTests = ([regex]::Matches($testOutput, "(\d+) passed")).Count
Write-Host "   Tests executed: Found $passedTests test results"