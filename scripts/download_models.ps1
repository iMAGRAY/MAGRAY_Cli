#!/usr/bin/env pwsh
# Script to download ONNX models for MAGRAY CLI

$ErrorActionPreference = "Stop"

Write-Host "📥 Downloading ONNX models for MAGRAY CLI..." -ForegroundColor Green

# Create models directory
$modelsDir = "models"
if (-not (Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir | Out-Null
    Write-Host "✅ Created models directory" -ForegroundColor Green
}

# Model configurations
$models = @(
    @{
        Name = "Qwen3-Embedding-0.6B-ONNX"
        Url = "https://huggingface.co/Alibaba-NLP/Qwen3-Embedding-0.6B-ONNX/resolve/main/model.onnx"
        Files = @(
            "model.onnx",
            "tokenizer.json",
            "tokenizer_config.json"
        )
    },
    @{
        Name = "Qwen3-Reranker-0.6B-ONNX"
        Url = "https://huggingface.co/Alibaba-NLP/Qwen3-Reranker-0.6B-ONNX/resolve/main/model.onnx"
        Files = @(
            "model.onnx",
            "tokenizer.json",
            "tokenizer_config.json"
        )
    }
)

foreach ($model in $models) {
    $modelPath = Join-Path $modelsDir $model.Name
    
    Write-Host "`n📦 Processing $($model.Name)..." -ForegroundColor Yellow
    
    # Create model directory
    if (-not (Test-Path $modelPath)) {
        New-Item -ItemType Directory -Path $modelPath | Out-Null
    }
    
    # Check if model already exists
    $modelFile = Join-Path $modelPath "model.onnx"
    if (Test-Path $modelFile) {
        Write-Host "⏭️  Model already exists, skipping..." -ForegroundColor Gray
        continue
    }
    
    Write-Host "⚠️  Note: You need to manually download model files from HuggingFace" -ForegroundColor Yellow
    Write-Host "   1. Visit: https://huggingface.co/Alibaba-NLP/$($model.Name)" -ForegroundColor White
    Write-Host "   2. Download these files to $modelPath :" -ForegroundColor White
    foreach ($file in $model.Files) {
        Write-Host "      - $file" -ForegroundColor Gray
    }
    Write-Host ""
}

Write-Host "`n📝 Manual download instructions:" -ForegroundColor Cyan
Write-Host "1. Install git-lfs if not already installed:" -ForegroundColor White
Write-Host "   winget install GitHub.GitLFS" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Clone models using git:" -ForegroundColor White
Write-Host "   cd models" -ForegroundColor Gray
Write-Host "   git clone https://huggingface.co/Alibaba-NLP/Qwen3-Embedding-0.6B-ONNX" -ForegroundColor Gray
Write-Host "   git clone https://huggingface.co/Alibaba-NLP/Qwen3-Reranker-0.6B-ONNX" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Or use huggingface-cli:" -ForegroundColor White
Write-Host "   pip install huggingface-hub" -ForegroundColor Gray
Write-Host "   huggingface-cli download Alibaba-NLP/Qwen3-Embedding-0.6B-ONNX --local-dir models/Qwen3-Embedding-0.6B-ONNX" -ForegroundColor Gray
Write-Host "   huggingface-cli download Alibaba-NLP/Qwen3-Reranker-0.6B-ONNX --local-dir models/Qwen3-Reranker-0.6B-ONNX" -ForegroundColor Gray
Write-Host ""

Write-Host "✅ Setup instructions complete!" -ForegroundColor Green