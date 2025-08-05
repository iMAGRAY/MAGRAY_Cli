#!/usr/bin/env bash
# Production warm-up script for MAGRAY CLI
# Прогревает критические компоненты перед production использованием

set -e  # Exit on error

echo -e "\033[36m🔥 MAGRAY CLI Production Warm-up Script\033[0m"
echo -e "\033[36m========================================\033[0m"

# Проверка что magray установлен
if ! command -v magray &> /dev/null; then
    echo -e "\033[31m❌ MAGRAY не найден в PATH. Установите его сначала.\033[0m"
    exit 1
fi

echo -e "\033[32m✅ MAGRAY найден: $(which magray)\033[0m"

# 1. Проверка системы
echo -e "\n\033[33m📊 Проверка системы...\033[0m"
magray status

# 2. Прогрев GPU (если доступен)
echo -e "\n\033[33m🎮 Проверка GPU...\033[0m"
magray gpu info || true  # Не критично если GPU недоступен

# 3. Инициализация памяти
echo -e "\n\033[33m💾 Инициализация системы памяти...\033[0m"
magray chat "This is a warm-up test to initialize memory indices and caches"

# 4. Проверка здоровья системы
echo -e "\n\033[33m🏥 Проверка здоровья системы...\033[0m"
magray health

# 5. Тест производительности
echo -e "\n\033[33m⚡ Тест производительности...\033[0m"
magray performance

# 6. Прогрев кэша эмбеддингов
echo -e "\n\033[33m🔄 Прогрев кэша эмбеддингов...\033[0m"
warmup_texts=(
    "Инициализация векторного поиска"
    "Тестирование системы памяти"
    "Проверка производительности индексов"
    "Warm-up для production использования"
    "Предварительная загрузка моделей"
)

for text in "${warmup_texts[@]}"; do
    echo -e "  \033[90m- Обработка: $text\033[0m"
    magray chat "$text" > /dev/null 2>&1
done

# 7. Финальная проверка
echo -e "\n\033[33m✨ Финальная проверка...\033[0m"
if magray status; then
    echo -e "\n\033[32m✅ Warm-up завершен успешно!\033[0m"
    echo -e "\033[32m🚀 MAGRAY готов к production использованию\033[0m"
else
    echo -e "\n\033[31m❌ Warm-up завершен с ошибками\033[0m"
    exit 1
fi

# Показываем итоговую статистику
echo -e "\n\033[36m📈 Итоговая статистика:\033[0m"
echo -e "\033[36m========================\033[0m"
magray memory stats

echo -e "\n\033[33m💡 Рекомендации:\033[0m"
echo "  - Запускайте этот скрипт после каждого перезапуска системы"
echo "  - Мониторьте производительность первых запросов"
echo "  - Используйте 'magray health' для регулярных проверок"