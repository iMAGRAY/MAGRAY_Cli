#!/usr/bin/env pwsh
# Production warm-up script for MAGRAY CLI
# Прогревает критические компоненты перед production использованием

Write-Host "🔥 MAGRAY CLI Production Warm-up Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Проверка что magray установлен
$magrayPath = Get-Command magray -ErrorAction SilentlyContinue
if (-not $magrayPath) {
    Write-Host "❌ MAGRAY не найден в PATH. Установите его сначала." -ForegroundColor Red
    exit 1
}

Write-Host "✅ MAGRAY найден: $($magrayPath.Path)" -ForegroundColor Green

# 1. Проверка системы
Write-Host "`n📊 Проверка системы..." -ForegroundColor Yellow
& magray status

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Ошибка при проверке статуса" -ForegroundColor Red
    exit 1
}

# 2. Прогрев GPU (если доступен)
Write-Host "`n🎮 Проверка GPU..." -ForegroundColor Yellow
& magray gpu info

# 3. Инициализация памяти
Write-Host "`n💾 Инициализация системы памяти..." -ForegroundColor Yellow
$testText = "This is a warm-up test to initialize memory indices and caches"
$result = & magray chat $testText

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Ошибка при инициализации памяти" -ForegroundColor Red
    exit 1
}

# 4. Проверка здоровья системы
Write-Host "`n🏥 Проверка здоровья системы..." -ForegroundColor Yellow
& magray health

# 5. Тест производительности
Write-Host "`n⚡ Тест производительности..." -ForegroundColor Yellow
& magray performance

# 6. Прогрев кэша эмбеддингов
Write-Host "`n🔄 Прогрев кэша эмбеддингов..." -ForegroundColor Yellow
$warmupTexts = @(
    "Инициализация векторного поиска",
    "Тестирование системы памяти",
    "Проверка производительности индексов",
    "Warm-up для production использования",
    "Предварительная загрузка моделей"
)

foreach ($text in $warmupTexts) {
    Write-Host "  - Обработка: $text" -ForegroundColor Gray
    $null = & magray chat $text 2>&1
}

# 7. Финальная проверка
Write-Host "`n✨ Финальная проверка..." -ForegroundColor Yellow
$finalStatus = & magray status

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ Warm-up завершен успешно!" -ForegroundColor Green
    Write-Host "🚀 MAGRAY готов к production использованию" -ForegroundColor Green
} else {
    Write-Host "`n❌ Warm-up завершен с ошибками" -ForegroundColor Red
    exit 1
}

# Показываем итоговую статистику
Write-Host "`n📈 Итоговая статистика:" -ForegroundColor Cyan
Write-Host "========================" -ForegroundColor Cyan
& magray memory stats

Write-Host "`n💡 Рекомендации:" -ForegroundColor Yellow
Write-Host "  - Запускайте этот скрипт после каждого перезапуска системы"
Write-Host "  - Мониторьте производительность первых запросов"
Write-Host "  - Используйте 'magray health' для регулярных проверок"