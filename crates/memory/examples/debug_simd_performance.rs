//! Debugging SIMD Performance Issues
//! 
//! Тестируем различные подходы к SIMD оптимизации для выяснения
//! почему производительность хуже скалярной версии

use memory::debug_simd_performance;

fn main() {
    debug_simd_performance();
}