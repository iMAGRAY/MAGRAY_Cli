#![cfg(not(feature = "minimal"))]

#[test]
fn test_dummy() {
    // Smoke placeholder to keep non-minimal builds exercising test runner
    assert_eq!(2 + 2, 4);
}
