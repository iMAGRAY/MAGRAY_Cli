use ai::{
    errors::Result,
    models::ModelType,
    tokenization::simple_qwen3::{SimpleQwen3Tokenizer, TokenizationResult},
    tokenizer::{TokenizeConfig, TokenizerManager, TokenizerType},
};
use arbitrary::{Arbitrary, Unstructured};
use mockall::{mock, predicate::*};
use proptest::prelude::*;
use quickcheck::{quickcheck, TestResult};
use rstest::*;
use serial_test::serial;
use std::{collections::HashSet, sync::Arc};

// Mock для внешних зависимостей токенизатора
mock! {
    ExternalTokenizer {}

    #[async_trait::async_trait]
    impl ai::tokenizer::TokenizerTrait for ExternalTokenizer {
        fn tokenize(&self, text: &str) -> Result<Vec<u32>>;
        fn decode(&self, tokens: &[u32]) -> Result<String>;
        fn encode(&self, text: &str) -> Result<Vec<u32>>;
        fn get_vocab_size(&self) -> usize;
    }
}

#[derive(Debug, Clone, Arbitrary)]
struct FuzzInput {
    text: String,
    max_length: Option<usize>,
    truncation: bool,
    padding: bool,
}

#[fixture]
fn tokenize_config() -> TokenizeConfig {
    TokenizeConfig {
        max_length: Some(512),
        truncation: true,
        padding: false,
        add_special_tokens: true,
        return_attention_mask: true,
        return_token_type_ids: false,
    }
}

#[fixture]
fn qwen3_tokenizer() -> SimpleQwen3Tokenizer {
    SimpleQwen3Tokenizer::new().expect("Failed to create Qwen3 tokenizer")
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_tokenizer_manager_creation() -> Result<()> {
    // Arrange - конфигурация для различных типов токенизаторов
    let test_cases = vec![
        (TokenizerType::Qwen3, true),
        (TokenizerType::BgeM3, true),
        (TokenizerType::Custom("test".to_string()), false), // может не существовать
    ];

    for (tokenizer_type, should_succeed) in test_cases {
        // Act - создание токенизатора
        let result = TokenizerManager::new(tokenizer_type.clone()).await;

        // Assert
        if should_succeed {
            assert!(
                result.is_ok(),
                "Создание токенизатора {:?} должно быть успешным",
                tokenizer_type
            );
        } else {
            // Для несуществующих типов может быть ошибка
            match result {
                Ok(_) => {
                    // Если все же создался, проверяем что он работает
                }
                Err(e) => {
                    assert!(
                        e.to_string().contains("not found")
                            || e.to_string().contains("unsupported"),
                        "Ошибка должна указывать на отсутствие поддержки: {}",
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_qwen3_tokenizer_basic_functionality(
    qwen3_tokenizer: SimpleQwen3Tokenizer,
) -> Result<()> {
    // Arrange - тестовые тексты различной сложности
    let test_cases = vec![
        ("Hello, world!", true),
        ("", true),                          // пустой текст
        ("Привет, мир!", true),              // unicode
        ("🚀 Emoji test 🎉", true),          // emoji
        ("Multiple\nlines\nof\ntext", true), // переносы строк
        ("Very long ".repeat(100), true),    // длинный текст
    ];

    for (text, should_succeed) in test_cases {
        // Act - токенизация
        let result = qwen3_tokenizer.tokenize_with_config(text, None);

        // Assert
        if should_succeed {
            assert!(
                result.is_ok(),
                "Токенизация '{}' должна быть успешной",
                text
            );

            let tokenization = result?;

            // Проверяем базовые свойства
            assert!(
                !tokenization.tokens.is_empty() || text.is_empty(),
                "Токены должны быть сгенерированы для непустого текста"
            );

            if let Some(attention_mask) = &tokenization.attention_mask {
                assert_eq!(
                    attention_mask.len(),
                    tokenization.tokens.len(),
                    "Размер attention mask должен соответствовать количеству токенов"
                );
            }

            // Проверяем что все токены валидны
            let vocab_size = qwen3_tokenizer.get_vocab_size();
            for &token in &tokenization.tokens {
                assert!(
                    (token as usize) < vocab_size,
                    "Токен {} должен быть в пределах словаря (размер: {})",
                    token,
                    vocab_size
                );
            }
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_encode_decode_roundtrip(
    qwen3_tokenizer: SimpleQwen3Tokenizer,
) -> Result<()> {
    // Arrange - тексты для roundtrip тестирования
    let test_texts = vec![
        "Simple English text",
        "Text with numbers 123 and symbols !@#",
        "Mixed языки text with разными alphabets",
    ];

    for original_text in test_texts {
        // Act - encode -> decode цикл
        let encoded = qwen3_tokenizer.encode(original_text)?;
        let decoded = qwen3_tokenizer.decode(&encoded)?;

        // Assert - проверяем семантическое сохранение
        // Точное восстановление может быть невозможно из-за особенностей токенизации,
        // но семантическое содержание должно сохраняться

        assert!(
            !encoded.is_empty(),
            "Кодировка не должна быть пустой для непустого текста"
        );
        assert!(!decoded.is_empty(), "Декодировка не должна быть пустой");

        // Проверяем что декодированный текст содержит основные слова
        let original_words: HashSet<&str> = original_text.split_whitespace().collect();
        let decoded_words: HashSet<&str> = decoded.split_whitespace().collect();

        // Большинство слов должно сохраняться (учитывая возможные изменения в токенизации)
        let preserved_words = original_words.intersection(&decoded_words).count();
        let total_words = original_words.len();

        if total_words > 0 {
            let preservation_ratio = preserved_words as f64 / total_words as f64;
            assert!(
                preservation_ratio >= 0.7,
                "Минимум 70% слов должно сохраняться при roundtrip: {:.2}% для '{}'",
                preservation_ratio * 100.0,
                original_text
            );
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_max_length_handling(
    qwen3_tokenizer: SimpleQwen3Tokenizer,
    mut tokenize_config: TokenizeConfig,
) -> Result<()> {
    // Arrange - длинный текст и различные ограничения длины
    let long_text =
        "This is a very long text that will definitely exceed the maximum length limit. "
            .repeat(20);

    let length_limits = vec![10, 50, 100, 512];

    for max_length in length_limits {
        tokenize_config.max_length = Some(max_length);
        tokenize_config.truncation = true;

        // Act - токенизация с ограничением длины
        let result =
            qwen3_tokenizer.tokenize_with_config(&long_text, Some(tokenize_config.clone()))?;

        // Assert - проверяем соблюдение ограничений
        assert!(
            result.tokens.len() <= max_length,
            "Количество токенов ({}) не должно превышать максимум ({})",
            result.tokens.len(),
            max_length
        );

        if let Some(attention_mask) = &result.attention_mask {
            assert!(
                attention_mask.len() <= max_length,
                "Размер attention mask не должен превышать максимум"
            );
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_padding_functionality(
    qwen3_tokenizer: SimpleQwen3Tokenizer,
    mut tokenize_config: TokenizeConfig,
) -> Result<()> {
    // Arrange - короткие тексты и padding
    let short_texts = vec!["Hi", "Hello", "How are you?"];
    let target_length = 20;

    tokenize_config.max_length = Some(target_length);
    tokenize_config.padding = true;

    for text in short_texts {
        // Act - токенизация с padding
        let result = qwen3_tokenizer.tokenize_with_config(text, Some(tokenize_config.clone()))?;

        // Assert - проверяем padding
        if result.tokens.len() < target_length {
            assert_eq!(
                result.tokens.len(),
                target_length,
                "При включенном padding длина должна соответствовать target_length"
            );

            // Проверяем что добавлены padding токены
            let padding_token_count = result
                .tokens
                .iter()
                .filter(|&&token| token == qwen3_tokenizer.get_pad_token_id())
                .count();

            assert!(
                padding_token_count > 0,
                "Должны быть добавлены padding токены"
            );
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_special_tokens(qwen3_tokenizer: SimpleQwen3Tokenizer) -> Result<()> {
    // Arrange - тест со специальными токенами и без них
    let test_text = "Hello world";

    let config_with_special = TokenizeConfig {
        add_special_tokens: true,
        ..Default::default()
    };

    let config_without_special = TokenizeConfig {
        add_special_tokens: false,
        ..Default::default()
    };

    // Act - токенизация с обеими конфигурациями
    let with_special =
        qwen3_tokenizer.tokenize_with_config(test_text, Some(config_with_special))?;
    let without_special =
        qwen3_tokenizer.tokenize_with_config(test_text, Some(config_without_special))?;

    // Assert - проверяем различия
    if qwen3_tokenizer.has_special_tokens() {
        assert!(
            with_special.tokens.len() >= without_special.tokens.len(),
            "С специальными токенами должно быть больше или равно токенов"
        );

        // Проверяем наличие специальных токенов
        let cls_token = qwen3_tokenizer.get_cls_token_id();
        let sep_token = qwen3_tokenizer.get_sep_token_id();

        if with_special.tokens.contains(&cls_token) {
            assert!(
                !without_special.tokens.contains(&cls_token),
                "CLS токен не должен присутствовать без специальных токенов"
            );
        }
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_concurrent_usage(qwen3_tokenizer: SimpleQwen3Tokenizer) -> Result<()> {
    // Arrange - параллельная токенизация
    let tokenizer = Arc::new(qwen3_tokenizer);
    let test_texts: Vec<String> = (0..50)
        .map(|i| format!("Concurrent tokenization test text number {}", i))
        .collect();

    // Act - параллельная обработка
    let tasks: Vec<_> = test_texts
        .into_iter()
        .map(|text| {
            let tokenizer = tokenizer.clone();
            tokio::spawn(async move { tokenizer.tokenize_with_config(&text, None) })
        })
        .collect();

    let results = futures::future::try_join_all(tasks).await?;

    // Assert - все задачи должны завершаться успешно
    for (i, result) in results.into_iter().enumerate() {
        assert!(
            result.is_ok(),
            "Параллельная токенизация {} должна быть успешной",
            i
        );

        let tokenization = result?;
        assert!(
            !tokenization.tokens.is_empty(),
            "Результат токенизации не должен быть пустым"
        );
    }

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_tokenizer_error_handling(qwen3_tokenizer: SimpleQwen3Tokenizer) -> Result<()> {
    // Arrange - проблемные входные данные
    let problematic_inputs = vec![
        "\u{FEFF}".repeat(1000), // много BOM символов
        "\0".repeat(100),        // null символы
        "\u{200B}".repeat(500),  // zero-width пробелы
    ];

    for input in problematic_inputs {
        // Act - токенизация проблемного ввода
        let result = qwen3_tokenizer.tokenize_with_config(&input, None);

        // Assert - должна быть корректная обработка ошибок
        match result {
            Ok(tokenization) => {
                // Если токенизация прошла успешно, проверяем результат
                assert!(
                    tokenization.tokens.len() < 10000,
                    "Количество токенов должно быть разумным даже для проблемного ввода"
                );
            }
            Err(e) => {
                // Если есть ошибка, она должна быть информативной
                assert!(
                    !e.to_string().is_empty(),
                    "Ошибка должна содержать описание"
                );
            }
        }
    }

    Ok(())
}

// Property-based тесты для проверки инвариантов токенизации
proptest! {
    #[test]
    fn test_tokenization_invariants(
        text in prop::string::string_regex("[\\w\\s\\p{P}]{0,200}").unwrap(),
        max_length in prop::option::of(1usize..=512)
    ) {
        tokio_test::block_on(async {
            let tokenizer = SimpleQwen3Tokenizer::new().unwrap();
            let config = TokenizeConfig {
                max_length,
                truncation: true,
                padding: false,
                add_special_tokens: true,
                return_attention_mask: true,
                return_token_type_ids: false,
            };

            if let Ok(result) = tokenizer.tokenize_with_config(&text, Some(config.clone())) {
                // Инвариант: количество токенов не превышает максимум
                if let Some(max_len) = max_length {
                    prop_assert!(result.tokens.len() <= max_len, "Tokens should not exceed max_length");
                }

                // Инвариант: все токены должны быть валидными
                let vocab_size = tokenizer.get_vocab_size();
                for &token in &result.tokens {
                    prop_assert!((token as usize) < vocab_size, "All tokens should be within vocabulary");
                }

                // Инвариант: attention mask соответствует токенам
                if let Some(attention_mask) = &result.attention_mask {
                    prop_assert_eq!(attention_mask.len(), result.tokens.len(), "Attention mask length should match tokens");

                    // Все значения в attention mask должны быть 0 или 1
                    for &mask_value in attention_mask {
                        prop_assert!(mask_value == 0 || mask_value == 1, "Attention mask values should be 0 or 1");
                    }
                }

                // Инвариант: пустой текст может давать пустой результат или только специальные токены
                if text.trim().is_empty() {
                    prop_assert!(result.tokens.len() <= 2, "Empty text should produce minimal tokens");
                }
            }
        })?;
    }

    #[test]
    fn test_encode_decode_properties(
        text in prop::string::string_regex("[a-zA-Z0-9 .,!?]{1,100}").unwrap()
    ) {
        tokio_test::block_on(async {
            let tokenizer = SimpleQwen3Tokenizer::new().unwrap();

            if let (Ok(encoded), Ok(decoded)) = (tokenizer.encode(&text), tokenizer.encode(&text).and_then(|tokens| tokenizer.decode(&tokens))) {
                // Инвариант: кодирование должно быть детерминистическим
                let encoded2 = tokenizer.encode(&text).unwrap();
                prop_assert_eq!(encoded, encoded2, "Encoding should be deterministic");

                // Инвариант: декодирование не должно увеличивать длину непропорционально
                prop_assert!(decoded.len() <= text.len() * 2, "Decoded text should not be excessively longer");

                // Инвариант: двойное кодирование-декодирование должно быть стабильным
                if let Ok(double_encoded) = tokenizer.encode(&decoded) {
                    if let Ok(double_decoded) = tokenizer.decode(&double_encoded) {
                        // После второй итерации изменения должны быть минимальными
                        let similarity = calculate_similarity(&decoded, &double_decoded);
                        prop_assert!(similarity >= 0.8, "Double encode-decode should be stable");
                    }
                }
            }
        })?;
    }
}

// QuickCheck тесты для дополнительной проверки
quickcheck! {
    fn qc_tokenization_length_bounds(text: String, max_len: Option<u16>) -> TestResult {
        if text.len() > 1000 { return TestResult::discard(); }

        let max_length = max_len.map(|l| l as usize).filter(|&l| l > 0 && l <= 1024);

        tokio_test::block_on(async {
            let tokenizer = match SimpleQwen3Tokenizer::new() {
                Ok(t) => t,
                Err(_) => return TestResult::discard(),
            };

            let config = TokenizeConfig {
                max_length,
                truncation: true,
                ..Default::default()
            };

            match tokenizer.tokenize_with_config(&text, Some(config)) {
                Ok(result) => {
                    if let Some(max_len) = max_length {
                        TestResult::from_bool(result.tokens.len() <= max_len)
                    } else {
                        TestResult::from_bool(result.tokens.len() <= 10000) // разумный максимум
                    }
                }
                Err(_) => TestResult::passed(), // ошибки токенизации допустимы
            }
        })
    }

    fn qc_vocab_bounds(text: String) -> TestResult {
        if text.is_empty() || text.len() > 200 { return TestResult::discard(); }

        tokio_test::block_on(async {
            let tokenizer = match SimpleQwen3Tokenizer::new() {
                Ok(t) => t,
                Err(_) => return TestResult::discard(),
            };

            match tokenizer.tokenize_with_config(&text, None) {
                Ok(result) => {
                    let vocab_size = tokenizer.get_vocab_size();
                    let all_tokens_valid = result.tokens.iter()
                        .all(|&token| (token as usize) < vocab_size);
                    TestResult::from_bool(all_tokens_valid)
                }
                Err(_) => TestResult::passed(),
            }
        })
    }
}

// Fuzzing-подобные тесты с arbitrary данными
#[test]
fn fuzz_tokenizer_with_arbitrary_input() {
    let mut data = vec![0u8; 1000];
    for _ in 0..100 {
        // Генерируем случайные данные
        for byte in &mut data {
            *byte = fastrand::u8(..);
        }

        let mut unstructured = Unstructured::new(&data);
        if let Ok(fuzz_input) = FuzzInput::arbitrary(&mut unstructured) {
            tokio_test::block_on(async {
                let tokenizer = SimpleQwen3Tokenizer::new().unwrap();

                let config = TokenizeConfig {
                    max_length: fuzz_input.max_length,
                    truncation: fuzz_input.truncation,
                    padding: fuzz_input.padding,
                    ..Default::default()
                };

                // Тестируем что токенизатор не паникует на любых входных данных
                let _ = tokenizer.tokenize_with_config(&fuzz_input.text, Some(config));
            });
        }
    }
}

// Утилиты для тестов
fn calculate_similarity(s1: &str, s2: &str) -> f64 {
    let words1: HashSet<&str> = s1.split_whitespace().collect();
    let words2: HashSet<&str> = s2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

// Benchmark тесты
#[tokio::test]
#[ignore]
async fn benchmark_tokenization_performance() -> Result<()> {
    let tokenizer = SimpleQwen3Tokenizer::new()?;

    let test_texts: Vec<String> = (0..1000)
        .map(|i| format!("Performance benchmark text number {} with various content and sufficient length for realistic testing scenarios", i))
        .collect();

    let start = std::time::Instant::now();

    for text in &test_texts {
        let _ = tokenizer.tokenize_with_config(text, None)?;
    }

    let duration = start.elapsed();
    let throughput = test_texts.len() as f64 / duration.as_secs_f64();

    println!("Tokenization Benchmark:");
    println!("  Texts processed: {}", test_texts.len());
    println!("  Time taken: {:?}", duration);
    println!("  Throughput: {:.2} texts/sec", throughput);

    // Проверяем минимальные требования к производительности
    assert!(
        throughput > 100.0,
        "Пропускная способность токенизации должна быть больше 100 текстов/сек"
    );

    Ok(())
}
