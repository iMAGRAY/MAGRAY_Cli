# Скрипт для загрузки ONNX Runtime GPU 1.22.0 с PyPI

$ErrorActionPreference = "Stop"

$downloadUrl = "https://files.pythonhosted.org/packages/ef/75/87b9b42cc8a97de821155432708a957e6ade42a94f9e34e9b7f6ad427c2f/onnxruntime_gpu-1.22.0-cp312-cp312-win_amd64.whl"
$outputFile = "onnxruntime_gpu-1.22.0.whl"
$extractDir = "onnxruntime_gpu_extracted"

Write-Host "Загрузка ONNX Runtime GPU 1.22.0..." -ForegroundColor Cyan
Write-Host "URL: $downloadUrl" -ForegroundColor Gray

# Загрузка файла
if (-not (Test-Path $outputFile)) {
    try {
        $webClient = New-Object System.Net.WebClient
        $webClient.DownloadFile($downloadUrl, $outputFile)
        Write-Host "✅ Загружено: $outputFile" -ForegroundColor Green
    } catch {
        Write-Host "❌ Ошибка загрузки: $_" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "✅ Файл уже существует: $outputFile" -ForegroundColor Green
}

# Распаковка wheel файла (это zip архив)
Write-Host "`nРаспаковка wheel файла..." -ForegroundColor Cyan

if (Test-Path $extractDir) {
    Remove-Item -Path $extractDir -Recurse -Force
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::ExtractToDirectory($outputFile, $extractDir)

Write-Host "✅ Распаковано в: $extractDir" -ForegroundColor Green

# Поиск DLL файлов
Write-Host "`nПоиск DLL файлов..." -ForegroundColor Cyan

$dllPath = Join-Path $extractDir "onnxruntime/capi"
if (Test-Path $dllPath) {
    $dlls = Get-ChildItem -Path $dllPath -Filter "*.dll"
    Write-Host "Найдено DLL файлов: $($dlls.Count)" -ForegroundColor Green
    $dlls | ForEach-Object { Write-Host "  - $($_.Name)" -ForegroundColor Gray }
    
    # Копирование в onnxruntime/lib
    $targetDir = ".\onnxruntime\lib"
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    
    Write-Host "`nКопирование DLL в $targetDir..." -ForegroundColor Cyan
    $dlls | Copy-Item -Destination $targetDir -Force
    Write-Host "✅ DLL файлы скопированы" -ForegroundColor Green
} else {
    Write-Host "❌ Путь $dllPath не найден" -ForegroundColor Red
}

Write-Host "`nГотово!" -ForegroundColor Green