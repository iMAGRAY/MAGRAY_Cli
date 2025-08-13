#![cfg(feature = "extended-tests")]

// =============================================================================
// ISOLATED BASIC TESTS - Минимальные изолированные тесты без зависимостей
// =============================================================================

// Тестируем только core types и functions которые должны работать
#[cfg(test)]
mod isolated_tests {
    use chrono::{DateTime, Utc};
    use serde_json;
    use uuid::Uuid;

    // Базовые тесты для типов, которые должны быть независимыми
    #[test]
    fn test_uuid_creation() {
        let id = Uuid::new_v4();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn test_datetime_creation() {
        let now = Utc::now();
        assert!(now.timestamp() > 0);
    }

    #[test]
    fn test_json_serialization() {
        let data = serde_json::json!({"test": "value"});
        let serialized = serde_json::to_string(&data).expect("Test operation should succeed");
        assert!(serialized.contains("test"));
    }

    // Тестируем базовые математические операции для векторов
    #[test]
    fn test_vector_dot_product() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];

        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();

        assert_eq!(dot_product, 32.0); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn test_vector_magnitude() {
        let vec1 = vec![3.0, 4.0];
        let magnitude = (vec1.iter().map(|x| x * x).sum::<f32>()).sqrt();
        assert_eq!(magnitude, 5.0); // √(3² + 4²) = 5
    }

    // Тест cosine distance функции если она доступна
    #[test]
    fn test_cosine_distance_manual() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0];

        // Cosine distance между перпендикулярными векторами должна быть 1.0
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let mag1: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag2: f32 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();

        let cosine_similarity = dot_product / (mag1 * mag2);
        let cosine_distance = 1.0 - cosine_similarity;

        assert!((cosine_distance - 1.0).abs() < 1e-6);
    }
}

// Property-based тесты с proptest
#[cfg(test)]
mod proptest_basic {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_vector_magnitude_property(vec in prop::collection::vec(any::<f32>(), 1..100)) {
            // Фильтруем NaN и infinite values
            let clean_vec: Vec<f32> = vec.into_iter()
                .filter(|x| x.is_finite())
                .collect();

            if !clean_vec.is_empty() {
                let magnitude = clean_vec.iter()
                    .map(|x| x * x)
                    .sum::<f32>()
                    .sqrt();

                // Magnitude должна быть неотрицательной и конечной
                prop_assert!(magnitude >= 0.0);
                prop_assert!(magnitude.is_finite());
            }
        }

        #[test]
        fn test_dot_product_commutative(
            vec1 in prop::collection::vec(any::<f32>(), 1..10),
            vec2 in prop::collection::vec(any::<f32>(), 1..10)
        ) {
            // Скалярное произведение коммутативно: a·b = b·a
            if vec1.len() == vec2.len() {
                let clean_vec1: Vec<f32> = vec1.into_iter().filter(|x| x.is_finite()).collect();
                let clean_vec2: Vec<f32> = vec2.into_iter().filter(|x| x.is_finite()).collect();

                if clean_vec1.len() == clean_vec2.len() && !clean_vec1.is_empty() {
                    let dot_ab: f32 = clean_vec1.iter().zip(clean_vec2.iter()).map(|(a, b)| a * b).sum();
                    let dot_ba: f32 = clean_vec2.iter().zip(clean_vec1.iter()).map(|(a, b)| a * b).sum();

                    if dot_ab.is_finite() && dot_ba.is_finite() {
                        prop_assert!((dot_ab - dot_ba).abs() < 1e-6);
                    }
                }
            }
        }
    }
}
