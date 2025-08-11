#![cfg(all(feature = "embeddings", feature = "extended-tests"))]

use ai::ort_setup::configure_ort_env;

#[test]
fn ort_env_respects_preexisting_var() {
    // If already set, function should not override
    std::env::set_var("ORT_DYLIB_PATH", "/tmp/libonnxruntime.so");
    configure_ort_env();
    let val = std::env::var("ORT_DYLIB_PATH").unwrap();
    assert_eq!(val, "/tmp/libonnxruntime.so");
}

#[test]
fn ort_env_no_panic_when_missing() {
    // Unset and ensure no panic
    std::env::remove_var("ORT_DYLIB_PATH");
    configure_ort_env();
    // Either set or not set — both acceptable; main point is no panic
}
