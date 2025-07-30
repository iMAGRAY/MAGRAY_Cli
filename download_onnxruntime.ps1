# PowerShell script to download ONNX Runtime
$ErrorActionPreference = "Stop"

Write-Host "Downloading ONNX Runtime for Windows x64..." -ForegroundColor Green

# ONNX Runtime version
$version = "1.18.1"
$url = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x64-gpu-$version.zip"
$tempFile = "onnxruntime.zip"
$targetDir = ".\onnxruntime"

# Create directory if not exists
if (!(Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir | Out-Null
}

# Download archive
Write-Host "Downloading from $url..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri $url -OutFile $tempFile -UseBasicParsing
} catch {
    Write-Host "GPU version download failed, trying CPU version..." -ForegroundColor Red
    $url = "https://github.com/microsoft/onnxruntime/releases/download/v$version/onnxruntime-win-x64-$version.zip"
    Invoke-WebRequest -Uri $url -OutFile $tempFile -UseBasicParsing
}

# Extract archive
Write-Host "Extracting archive..." -ForegroundColor Yellow
Expand-Archive -Path $tempFile -DestinationPath "temp_ort" -Force

# Copy files
$sourceDir = Get-ChildItem -Path "temp_ort" -Directory | Select-Object -First 1
if ($sourceDir) {
    Write-Host "Copying files..." -ForegroundColor Yellow
    
    # Copy lib files
    $libSource = Join-Path $sourceDir.FullName "lib"
    $libTarget = Join-Path $targetDir "lib"
    if (!(Test-Path $libTarget)) {
        New-Item -ItemType Directory -Path $libTarget | Out-Null
    }
    Copy-Item -Path "$libSource\*" -Destination $libTarget -Force -Recurse
    
    # Copy include files if exists
    $includeSource = Join-Path $sourceDir.FullName "include"
    if (Test-Path $includeSource) {
        $includeTarget = Join-Path $targetDir "include"
        if (!(Test-Path $includeTarget)) {
            New-Item -ItemType Directory -Path $includeTarget | Out-Null
        }
        Copy-Item -Path "$includeSource\*" -Destination $includeTarget -Force -Recurse
    }
}

# Clean temp files
Write-Host "Cleaning temp files..." -ForegroundColor Yellow
Remove-Item -Path $tempFile -Force
Remove-Item -Path "temp_ort" -Recurse -Force

Write-Host "ONNX Runtime installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Add to environment variables:" -ForegroundColor Cyan
Write-Host "   ORT_DYLIB_PATH=$pwd\onnxruntime\lib\onnxruntime.dll" -ForegroundColor White
Write-Host ""
Write-Host "Or run:" -ForegroundColor Cyan
Write-Host "   `$env:ORT_DYLIB_PATH = `"$pwd\onnxruntime\lib\onnxruntime.dll`"" -ForegroundColor White