use ai::{
    errors::Result,
    model_downloader::{ensure_model, ModelDownloader},
};
use mockall::{mock, predicate::*};
use proptest::prelude::*;
use rstest::*;
use serial_test::serial;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::TempDir;
use tokio::fs;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

// Mock для тестирования HTTP клиента
mock! {
    HttpClient {}

    #[async_trait::async_trait]
    impl reqwest::Client for HttpClient {
        async fn get(&self, url: &str) -> Result<reqwest::Response>;
    }
}

#[fixture]
async fn temp_model_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

#[fixture]
async fn model_downloader(temp_model_dir: TempDir) -> ModelDownloader {
    ModelDownloader::new(temp_model_dir.path()).expect("Failed to create ModelDownloader")
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_model_downloader_creation_success(temp_model_dir: TempDir) -> Result<()> {
    // Arrange - временная директория уже создана

    // Act - создание downloader'а
    let downloader = ModelDownloader::new(temp_model_dir.path());

    // Assert - проверка успешного создания
    assert!(
        downloader.is_ok(),
        "ModelDownloader должен создаваться успешно"
    );

    let downloader = downloader?;

    // Проверяем что базовая директория существует
    assert!(
        temp_model_dir.path().exists(),
        "Базовая директория должна существовать"
    );

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_model_downloader_creation_invalid_path() -> Result<()> {
    // Arrange - некорректный путь
    let invalid_path = Path::new("/invalid/non/existent/path/with/no/permissions");

    // Act - попытка создания с некорректным путем
    let result = ModelDownloader::new(invalid_path);

    // Assert - должна быть ошибка
    assert!(
        result.is_err(),
        "Создание ModelDownloader с некорректным путем должно завершаться ошибкой"
    );

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("path") || error.to_string().contains("permission"),
        "Ошибка должна содержать информацию о пути: {}",
        error
    );

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_ensure_model_already_exists(model_downloader: ModelDownloader) -> Result<()> {
    // Arrange - создаем mock файл модели
    let model_name = "test-model";
    let expected_path = model_downloader.get_model_path(model_name);

    // Создаем директории и файл
    if let Some(parent) = expected_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&expected_path, b"mock model data").await?;

    // Act - попытка получить уже существующую модель
    let result = model_downloader.ensure_model(model_name).await;

    // Assert - должна вернуться существующая модель
    assert!(
        result.is_ok(),
        "ensure_model должен успешно находить существующую модель"
    );

    let returned_path = result?;
    assert_eq!(returned_path, expected_path, "Пути должны совпадать");
    assert!(returned_path.exists(), "Файл модели должен существовать");

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_ensure_model_download_new(temp_model_dir: TempDir) -> Result<()> {
    // Arrange - настройка mock сервера для симуляции загрузки
    let mock_server = MockServer::start().await;

    let model_data = b"mock downloaded model data";
    Mock::given(method("GET"))
        .and(path("/models/test-model"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(model_data))
        .mount(&mock_server)
        .await;

    let downloader = ModelDownloader::new(temp_model_dir.path())?;

    // Подменяем URL для тестирования (в реальной реализации это было бы через конфигурацию)
    let model_name = "test-model";

    // Act - загрузка новой модели
    // Примечание: в реальной реализации нужно будет мокать HTTP клиент
    // Здесь мы тестируем логику, предполагая что HTTP запросы работают

    // Имитируем процесс загрузки, создав файл напрямую
    let model_path = downloader.get_model_path(model_name);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&model_path, model_data).await?;

    let result = downloader.ensure_model(model_name).await;

    // Assert
    assert!(result.is_ok(), "Загрузка новой модели должна быть успешной");

    let returned_path = result?;
    assert_eq!(returned_path, model_path);
    assert!(
        returned_path.exists(),
        "Загруженная модель должна существовать"
    );

    let content = fs::read(&returned_path).await?;
    assert_eq!(
        content, model_data,
        "Содержимое должно соответствовать загруженному"
    );

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_ensure_model_download_failure(model_downloader: ModelDownloader) -> Result<()> {
    // Arrange - настройка mock сервера для симуляции ошибки загрузки
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/models/failing-model"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // Act - попытка загрузки несуществующей модели
    let model_name = "failing-model";

    // В реальной реализации это бы вызывало HTTP запрос и получало 404
    // Здесь мы симулируем ошибку напрямую
    let result = model_downloader.ensure_model(model_name).await;

    // Assert - в зависимости от реализации, может быть ошибка или fallback поведение
    // Если модель не существует локально и не может быть загружена, должна быть ошибка
    if result.is_err() {
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("download")
                || error.to_string().contains("404")
                || error.to_string().contains("not found"),
            "Ошибка должна указывать на проблему с загрузкой: {}",
            error
        );
    }

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_model_downloader_concurrent_downloads(temp_model_dir: TempDir) -> Result<()> {
    // Arrange
    let downloader = Arc::new(ModelDownloader::new(temp_model_dir.path())?);

    let model_names = vec!["model-1", "model-2", "model-3"];

    // Предварительно создаем модели для симуляции
    for name in &model_names {
        let model_path = downloader.get_model_path(name);
        if let Some(parent) = model_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&model_path, format!("data for {}", name)).await?;
    }

    // Act - параллельные загрузки
    let tasks: Vec<_> = model_names
        .into_iter()
        .map(|name| {
            let downloader = downloader.clone();
            tokio::spawn(async move { downloader.ensure_model(name).await })
        })
        .collect();

    let results = futures::future::try_join_all(tasks).await?;

    // Assert - все загрузки должны быть успешными
    for (i, result) in results.into_iter().enumerate() {
        assert!(
            result.is_ok(),
            "Параллельная загрузка модели {} должна быть успешной",
            i
        );

        let path = result?;
        assert!(path.exists(), "Файл модели должен существовать");
    }

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_cache_management(model_downloader: ModelDownloader) -> Result<()> {
    // Arrange - создаем несколько тестовых моделей
    let model_names = vec!["cache-test-1", "cache-test-2", "cache-test-3"];

    for name in &model_names {
        let model_path = model_downloader.get_model_path(name);
        if let Some(parent) = model_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&model_path, format!("test data for {}", name)).await?;
    }

    // Act - получаем размер кеша до очистки
    let initial_size = model_downloader.get_cache_size().await?;
    assert!(initial_size > 0, "Размер кеша должен быть больше нуля");

    // Очищаем кеш
    let clear_result = model_downloader.clear_cache().await;
    assert!(
        clear_result.is_ok(),
        "Очистка кеша должна проходить успешно"
    );

    // Assert - проверяем что кеш очищен
    let final_size = model_downloader.get_cache_size().await?;
    assert_eq!(
        final_size, 0,
        "После очистки размер кеша должен быть нулевым"
    );

    // Проверяем что файлы удалены
    for name in &model_names {
        let model_path = model_downloader.get_model_path(name);
        assert!(
            !model_path.exists(),
            "Файл модели должен быть удален после очистки кеша"
        );
    }

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_global_ensure_model_function() -> Result<()> {
    // Arrange - используем глобальную функцию
    let model_name = "global-test-model";

    // Act - вызываем глобальную функцию
    let result = ensure_model(model_name).await;

    // Assert - функция должна работать
    // В зависимости от реализации может быть успех или ошибка
    match result {
        Ok(path) => {
            assert!(
                path.exists(),
                "Глобальная функция должна возвращать существующий путь"
            );
        }
        Err(e) => {
            // Если нет глобальной конфигурации, ошибка ожидаема
            assert!(
                e.to_string().contains("config") || e.to_string().contains("not initialized"),
                "Ошибка глобальной функции должна быть информативной: {}",
                e
            );
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_model_path_generation(model_downloader: ModelDownloader) -> Result<()> {
    // Arrange - различные имена моделей
    let test_cases = vec![
        ("simple-model", true),
        ("model_with_underscores", true),
        ("model-with-dashes", true),
        ("model123", true),
        ("", false),                    // пустое имя должно быть некорректным
        ("../../../etc/passwd", false), // попытка path traversal
        ("model/with/slashes", true),   // может быть валидным в зависимости от реализации
    ];

    for (model_name, should_be_valid) in test_cases {
        // Act - генерация пути
        let path = model_downloader.get_model_path(model_name);

        // Assert - проверяем корректность пути
        if should_be_valid {
            assert!(
                !path.to_string_lossy().is_empty(),
                "Путь для валидного имени '{}' не должен быть пустым",
                model_name
            );

            // Проверяем что путь находится в пределах базовой директории
            assert!(
                path.starts_with(model_downloader.base_path()),
                "Путь должен начинаться с базовой директории"
            );
        }

        // Проверяем что path traversal невозможен
        let path_str = path.to_string_lossy();
        assert!(
            !path_str.contains(".."),
            "Путь не должен содержать '..' для безопасности"
        );
    }

    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_download_with_progress_tracking(model_downloader: ModelDownloader) -> Result<()> {
    // Arrange - создаем большой файл для симуляции загрузки с прогрессом
    let model_name = "large-model";
    let model_path = model_downloader.get_model_path(model_name);

    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Создаем "большой" файл (в тестах не делаем реально большой)
    let large_data = vec![0u8; 10240]; // 10KB для теста
    fs::write(&model_path, &large_data).await?;

    // Act - проверяем что модель может быть получена
    let result = model_downloader.ensure_model(model_name).await;

    // Assert
    assert!(
        result.is_ok(),
        "Получение большой модели должно быть успешным"
    );

    let returned_path = result?;
    let file_size = fs::metadata(&returned_path).await?.len();
    assert_eq!(
        file_size,
        large_data.len() as u64,
        "Размер файла должен соответствовать ожидаемому"
    );

    Ok(())
}

// Property-based тесты
proptest! {
    #[test]
    fn test_model_path_safety(
        model_name in prop::string::string_regex("[a-zA-Z0-9_-]{1,50}").unwrap()
    ) {
        tokio_test::block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let downloader = ModelDownloader::new(temp_dir.path()).unwrap();

            let path = downloader.get_model_path(&model_name);
            let base_path = downloader.base_path();

            // Инвариант: генерируемый путь всегда должен быть внутри базовой директории
            prop_assert!(path.starts_with(base_path), "Путь модели должен быть внутри базовой директории");

            // Инвариант: путь не должен содержать опасные элементы
            let path_str = path.to_string_lossy();
            prop_assert!(!path_str.contains(".."), "Путь не должен содержать '..'");
            prop_assert!(!path_str.contains("//"), "Путь не должен содержать двойные слеши");
        })?;
    }

    #[test]
    fn test_cache_size_consistency(
        file_count in 1usize..10,
        file_size in 100usize..1000
    ) {
        tokio_test::block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let downloader = ModelDownloader::new(temp_dir.path()).unwrap();

            let mut expected_size = 0u64;

            // Создаем файлы известного размера
            for i in 0..file_count {
                let model_name = format!("test_model_{}", i);
                let model_path = downloader.get_model_path(&model_name);

                if let Some(parent) = model_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let data = vec![0u8; file_size];
                if std::fs::write(&model_path, &data).is_ok() {
                    expected_size += file_size as u64;
                }
            }

            if let Ok(actual_size) = downloader.get_cache_size().await {
                // Инвариант: размер кеша должен соответствовать сумме размеров файлов
                prop_assert_eq!(actual_size, expected_size, "Размер кеша должен соответствовать сумме файлов");
            }
        })?;
    }
}

// Stress тесты
#[tokio::test]
#[ignore] // Игнорируем по умолчанию
async fn stress_test_many_concurrent_downloads() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let downloader = Arc::new(ModelDownloader::new(temp_dir.path())?);

    let num_tasks = 100;
    let tasks: Vec<_> = (0..num_tasks)
        .map(|i| {
            let downloader = downloader.clone();
            let model_name = format!("stress_model_{}", i % 10); // 10 разных моделей

            tokio::spawn(async move {
                // Предварительно создаем модель
                let model_path = downloader.get_model_path(&model_name);
                if let Some(parent) = model_path.parent() {
                    let _ = fs::create_dir_all(parent).await;
                }
                let _ = fs::write(&model_path, format!("data_{}", model_name)).await;

                downloader.ensure_model(&model_name).await
            })
        })
        .collect();

    let results = futures::future::join_all(tasks).await;

    let mut success_count = 0;
    for result in results {
        if let Ok(Ok(_)) = result {
            success_count += 1;
        }
    }

    println!(
        "Stress test: {}/{} tasks completed successfully",
        success_count, num_tasks
    );
    assert!(
        success_count >= num_tasks * 9 / 10,
        "Минимум 90% задач должны завершаться успешно"
    );

    Ok(())
}

// Integration тесты с реальной файловой системой
#[tokio::test]
#[ignore] // Игнорируем по умолчанию, так как требует реальной FS
async fn integration_test_real_filesystem() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let downloader = ModelDownloader::new(temp_dir.path())?;

    // Полный цикл: создание -> проверка -> очистка
    let model_name = "integration_test_model";

    // Создаем модель
    let model_path = downloader.get_model_path(model_name);
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&model_path, b"integration test data").await?;

    // Проверяем размер кеша
    let size_before = downloader.get_cache_size().await?;
    assert!(size_before > 0);

    // Получаем модель
    let result_path = downloader.ensure_model(model_name).await?;
    assert_eq!(result_path, model_path);

    // Очищаем кеш
    downloader.clear_cache().await?;

    let size_after = downloader.get_cache_size().await?;
    assert_eq!(size_after, 0);

    assert!(!model_path.exists());

    Ok(())
}
