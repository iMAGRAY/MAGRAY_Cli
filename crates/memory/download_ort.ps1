# PowerShell script to download ONNX Runtime 1.22.2 for Windows x64

$ortVersion = "1.22.0"
$ortUrl = "https://github.com/microsoft/onnxruntime/releases/download/v$ortVersion/onnxruntime-win-x64-$ortVersion.zip"
$targetDir = "target\ort-libs"
$zipFile = "$targetDir\onnxruntime.zip"

Write-Host "Downloading ONNX Runtime $ortVersion..." -ForegroundColor Green

# Create target directory
if (!(Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}

# Download the zip file
try {
    Invoke-WebRequest -Uri $ortUrl -OutFile $zipFile
    Write-Host "Downloaded successfully!" -ForegroundColor Green
} catch {
    Write-Host "Failed to download: $_" -ForegroundColor Red
    exit 1
}

# Extract the zip file
Write-Host "Extracting..." -ForegroundColor Yellow
Expand-Archive -Path $zipFile -DestinationPath $targetDir -Force

# Move DLLs to the right place
$ortExtractedDir = "$targetDir\onnxruntime-win-x64-$ortVersion"
if (Test-Path $ortExtractedDir) {
    $libDir = "$ortExtractedDir\lib"
    if (Test-Path $libDir) {
        Write-Host "Copying DLLs..." -ForegroundColor Yellow
        Copy-Item "$libDir\*.dll" -Destination $targetDir -Force
        Copy-Item "$libDir\*.dll" -Destination "target\debug" -Force -ErrorAction SilentlyContinue
        
        Write-Host "ONNX Runtime DLLs installed to:" -ForegroundColor Green
        Write-Host "  - $targetDir" -ForegroundColor Cyan
        Write-Host "  - target\debug" -ForegroundColor Cyan
    }
}

# Clean up
Remove-Item $zipFile -Force -ErrorAction SilentlyContinue
Remove-Item $ortExtractedDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "`nDone! ONNX Runtime $ortVersion is ready to use." -ForegroundColor Green