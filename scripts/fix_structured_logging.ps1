#!/usr/bin/env pwsh
# Script to fix structured_logging.rs by removing merge conflicts

$filePath = "..\crates\common\src\structured_logging.rs"
$content = Get-Content $filePath -Raw

# Remove all merge conflict markers, keeping HEAD version
$content = $content -replace '(?s)<<<<<<< HEAD\r?\n(.*?)\r?\n=======\r?\n.*?\r?\n>>>>>>> [^\r\n]+', '$1'

# Fix specific formatting issues
$content = $content -replace '\{json\}', '{$json}'
$content = $content -replace '\{value:?\}', '{:?}", value'
$content = $content -replace '\$\(format!\{:?}", value\)', 'format!("{:?}", value)'

# Save the fixed content
$content | Set-Content -Path $filePath -NoNewline -Encoding UTF8

Write-Host "Fixed structured_logging.rs"