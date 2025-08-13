/// P1.2.1.c: WASM Execution Tests
///
/// Comprehensive tests for WASM function execution and result handling
use tools::wasm_runtime::*;

#[cfg(test)]
mod execution_tests {
    use super::*;

    /// Create a minimal WASM module for testing
    /// This is a hand-crafted WASM module with simple functions
    fn create_test_wasm_module() -> Vec<u8> {
        // Minimal WASM module with add(i32, i32) -> i32 function
        // This is a real WASM bytecode for: (module (func (export "add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))
        vec![
            0x00, 0x61, 0x73, 0x6d, // WASM magic number
            0x01, 0x00, 0x00, 0x00, // Version 1
            0x01, 0x07, // Type section
            0x01, // 1 type
            0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // func type: (i32, i32) -> i32
            0x03, 0x02, // Function section
            0x01, // 1 function
            0x00, // function 0, type 0
            0x07, 0x07, // Export section
            0x01, // 1 export
            0x03, 0x61, 0x64, 0x64, // "add"
            0x00, 0x00, // function export, function 0
            0x0a, 0x09, // Code section
            0x01, // 1 function body
            0x07, // body size
            0x00, // 0 locals
            0x20, 0x00, // local.get 0
            0x20, 0x01, // local.get 1
            0x6a, // i32.add
            0x0b, // end
        ]
    }

    /// Create another test WASM module with multiply function
    fn create_multiply_wasm_module() -> Vec<u8> {
        // WASM module with multiply(i32, i32) -> i32 function
        vec![
            0x00, 0x61, 0x73, 0x6d, // WASM magic number
            0x01, 0x00, 0x00, 0x00, // Version 1
            0x01, 0x07, // Type section
            0x01, // 1 type
            0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f, // func type: (i32, i32) -> i32
            0x03, 0x02, // Function section
            0x01, // 1 function
            0x00, // function 0, type 0
            0x07, 0x0c, // Export section
            0x01, // 1 export
            0x08, 0x6d, 0x75, 0x6c, 0x74, 0x69, 0x70, 0x6c, 0x79, // "multiply"
            0x00, 0x00, // function export, function 0
            0x0a, 0x09, // Code section
            0x01, // 1 function body
            0x07, // body size
            0x00, // 0 locals
            0x20, 0x00, // local.get 0
            0x20, 0x01, // local.get 1
            0x6c, // i32.mul
            0x0b, // end
        ]
    }

    #[tokio::test]
    async fn test_execute_function_success() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            let runtime = runtime.expect("Test operation should succeed");
            let wasm_bytes = create_test_wasm_module();
            let module = runtime
                .load_module_from_bytes(&wasm_bytes)
                .expect("Test operation should succeed");

            // Execute add function
            let params = vec![WasmValue::I32(5), WasmValue::I32(3)];
            let result = runtime
                .execute_function(&module, "add", params.clone())
                .await
                .expect("Test operation should succeed");

            assert!(result.success);
            assert_eq!(result.function_name, "add");
            assert_eq!(result.input_params, params);
            assert_eq!(result.return_values.len(), 1);

            if let WasmValue::I32(sum) = &result.return_values[0] {
                assert_eq!(*sum, 8); // 5 + 3 = 8
            } else {
                panic!("Expected I32 return value");
            }

            assert!(result.execution_time_us > 0);
            assert!(result.error_message.is_none());
        }

        #[cfg(not(feature = "wasm-runtime"))]
        {
            assert!(runtime.is_err());
        }
    }

    #[tokio::test]
    async fn test_execute_multiply_function() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            let runtime = runtime.expect("Test operation should succeed");
            let wasm_bytes = create_multiply_wasm_module();
            let module = runtime
                .load_module_from_bytes(&wasm_bytes)
                .expect("Test operation should succeed");

            // Execute multiply function
            let params = vec![WasmValue::I32(7), WasmValue::I32(6)];
            let result = runtime
                .execute_function(&module, "multiply", params.clone())
                .await
                .expect("Test operation should succeed");

            assert!(result.success);
            assert_eq!(result.function_name, "multiply");
            assert_eq!(result.input_params, params);
            assert_eq!(result.return_values.len(), 1);

            if let WasmValue::I32(product) = &result.return_values[0] {
                assert_eq!(*product, 42); // 7 * 6 = 42
            } else {
                panic!("Expected I32 return value");
            }
        }
    }

    #[tokio::test]
    async fn test_execute_function_not_found() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            let runtime = runtime.expect("Test operation should succeed");
            let wasm_bytes = create_test_wasm_module();
            let module = runtime
                .load_module_from_bytes(&wasm_bytes)
                .expect("Test operation should succeed");

            // Try to execute non-existent function
            let params = vec![WasmValue::I32(1), WasmValue::I32(2)];
            let result = runtime
                .execute_function(&module, "nonexistent", params.clone())
                .await
                .expect("Test operation should succeed");

            assert!(!result.success);
            assert_eq!(result.function_name, "nonexistent");
            assert_eq!(result.input_params, params);
            assert!(result.return_values.is_empty());
            assert!(result.error_message.is_some());
            assert!(result
                .error_message
                .expect("Test operation should succeed")
                .contains("Function 'nonexistent' not found"));
        }
    }

    #[test]
    fn test_wasm_value_conversion() {
        #[cfg(feature = "wasm-runtime")]
        {
            // Test I32 conversion
            let val_i32 = WasmValue::I32(42);
            let wasmtime_val = val_i32.to_wasmtime_val();
            let converted_back =
                WasmValue::from_wasmtime_val(&wasmtime_val).expect("Test operation should succeed");
            assert!(matches!(converted_back, WasmValue::I32(42)));

            // Test I64 conversion
            let val_i64 = WasmValue::I64(1234567890);
            let wasmtime_val = val_i64.to_wasmtime_val();
            let converted_back =
                WasmValue::from_wasmtime_val(&wasmtime_val).expect("Test operation should succeed");
            assert!(matches!(converted_back, WasmValue::I64(1234567890)));

            // Test F32 conversion
            let val_f32 = WasmValue::F32(std::f32::consts::PI);
            let wasmtime_val = val_f32.to_wasmtime_val();
            let converted_back =
                WasmValue::from_wasmtime_val(&wasmtime_val).expect("Test operation should succeed");
            if let WasmValue::F32(f) = converted_back {
                assert!((f - std::f32::consts::PI).abs() < 0.001);
            } else {
                panic!("Expected F32");
            }

            // Test F64 conversion
            let val_f64 = WasmValue::F64(std::f64::consts::E);
            let wasmtime_val = val_f64.to_wasmtime_val();
            let converted_back =
                WasmValue::from_wasmtime_val(&wasmtime_val).expect("Test operation should succeed");
            if let WasmValue::F64(f) = converted_back {
                assert!((f - std::f64::consts::E).abs() < 0.000000001);
            } else {
                panic!("Expected F64");
            }
        }
    }

    #[test]
    fn test_execution_result_creation() {
        // Test successful result
        let params = vec![WasmValue::I32(10), WasmValue::I32(20)];
        let returns = vec![WasmValue::I32(30)];
        let result = WasmExecutionResult::success(
            "test_func".to_string(),
            params.clone(),
            returns.clone(),
            1000,
            4096,
        );

        assert!(result.success);
        assert_eq!(result.function_name, "test_func");
        assert_eq!(result.input_params, params);
        assert_eq!(result.return_values, returns);
        assert_eq!(result.execution_time_us, 1000);
        assert_eq!(result.memory_usage_bytes, 4096);
        assert!(result.error_message.is_none());

        // Test failure result
        let failure_result = WasmExecutionResult::failure(
            "failed_func".to_string(),
            params.clone(),
            "Test error".to_string(),
            500,
        );

        assert!(!failure_result.success);
        assert_eq!(failure_result.function_name, "failed_func");
        assert_eq!(failure_result.input_params, params);
        assert!(failure_result.return_values.is_empty());
        assert_eq!(failure_result.execution_time_us, 500);
        assert_eq!(failure_result.memory_usage_bytes, 0);
        assert_eq!(failure_result.error_message, Some("Test error".to_string()));
    }

    #[tokio::test]
    async fn test_function_signature_validation() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            let runtime = runtime.expect("Test operation should succeed");
            let wasm_bytes = create_test_wasm_module();
            let module = runtime
                .load_module_from_bytes(&wasm_bytes)
                .expect("Test operation should succeed");

            // Test function existence
            assert!(module.has_function("add"));
            assert!(!module.has_function("subtract"));

            // Test function signature
            let signature = module.get_function_signature("add");
            assert!(signature.is_some());

            let (params, results) = signature.expect("Test operation should succeed");
            assert_eq!(params.len(), 2); // Two i32 parameters
            assert_eq!(results.len(), 1); // One i32 result
        }
    }

    #[tokio::test]
    async fn test_execution_performance_tracking() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            let runtime = runtime.expect("Test operation should succeed");
            let wasm_bytes = create_test_wasm_module();
            let module = runtime
                .load_module_from_bytes(&wasm_bytes)
                .expect("Test operation should succeed");

            // Execute multiple times to test performance consistency
            for i in 0..5 {
                let params = vec![WasmValue::I32(i * 2), WasmValue::I32(i * 3)];
                let result = runtime
                    .execute_function(&module, "add", params)
                    .await
                    .expect("Test operation should succeed");

                assert!(result.success);
                assert!(result.execution_time_us > 0);
                assert!(result.execution_time_us < 1_000_000); // Should be under 1 second

                // Verify calculation
                if let WasmValue::I32(sum) = &result.return_values[0] {
                    assert_eq!(*sum, i * 2 + i * 3);
                }
            }
        }
    }

    #[test]
    fn test_wasm_runtime_error_types() {
        // Test that all error types can be created and formatted
        let errors = vec![
            WasmRuntimeError::EngineInit("test".to_string()),
            WasmRuntimeError::Compilation("test".to_string()),
            WasmRuntimeError::Instantiation("test".to_string()),
            WasmRuntimeError::Validation("test".to_string()),
            WasmRuntimeError::WasiSetup("test".to_string()),
            WasmRuntimeError::ResourceLimit("test".to_string()),
            WasmRuntimeError::Execution("test".to_string()),
            WasmRuntimeError::ModuleLoad("test".to_string()),
            WasmRuntimeError::FeatureNotAvailable("test".to_string()),
            WasmRuntimeError::FunctionNotFound("test".to_string()),
            WasmRuntimeError::ParameterMismatch("test".to_string()),
            WasmRuntimeError::TypeConversion("test".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.contains("test"));
        }
    }

    #[tokio::test]
    async fn test_feature_flag_behavior() {
        let runtime = WasmRuntime::with_defaults();

        #[cfg(feature = "wasm-runtime")]
        {
            // When feature is enabled, everything should work
            assert!(runtime.is_ok());
            let runtime = runtime.expect("Test operation should succeed");

            let wasm_bytes = create_test_wasm_module();
            let module = runtime.load_module_from_bytes(&wasm_bytes);
            assert!(module.is_ok());
        }

        #[cfg(not(feature = "wasm-runtime"))]
        {
            // When feature is disabled, should get FeatureNotAvailable errors
            assert!(runtime.is_err());
            match runtime {
                Err(WasmRuntimeError::FeatureNotAvailable(_)) => {} // Expected
                _ => panic!("Expected FeatureNotAvailable error"),
            }
        }
    }
}
