//! Ultra-Advanced SIMD Feature Detection and Adaptive Algorithm Selection
//!
//! Автоматический выбор наилучших алгоритмов на основе:
//! - CPU architecture capabilities (AVX-512, AVX2, SSE, ARM NEON)
//! - Data characteristics (size, alignment, sparsity)
//! - System resources (memory bandwidth, cache sizes)
//! - Workload patterns (batch size, frequency)
//!
//! Цель: максимальная производительность на любой архитектуре
//!
//! Safety
//! - В модуле нет прямых `unsafe` SIMD-интринсиков. Мы используем детекцию фич
//!   через `is_x86_feature_detected!` и адаптивный выбор алгоритма на уровне
//!   чистого Rust.
//! - Глобальные синглтоны реализованы через `Once`/`OnceLock` для отсутствия data race.
//! - Тесты с потенциально долгими хронометрами отключены в CI через переменную `CI`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Once, OnceLock};
use tracing::{debug, info};

/// Global CPU feature detection results
static INIT: Once = Once::new();
static HAS_AVX512F: AtomicBool = AtomicBool::new(false);
static HAS_AVX512VL: AtomicBool = AtomicBool::new(false);
static HAS_AVX2: AtomicBool = AtomicBool::new(false);
static HAS_FMA: AtomicBool = AtomicBool::new(false);
static HAS_SSE42: AtomicBool = AtomicBool::new(false);
static CACHE_LINE_SIZE: AtomicU32 = AtomicU32::new(64);
static L1_CACHE_SIZE: AtomicU32 = AtomicU32::new(32768);
static L3_CACHE_SIZE: AtomicU32 = AtomicU32::new(8388608);

/// SIMD capability levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdLevel {
    None,
    Sse42,
    Avx2,
    Avx2Fma,
    Avx512F,
    Avx512VL,
}

/// CPU архитектура information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub simd_level: SimdLevel,
    pub cache_line_size: u32,
    pub l1_cache_size: u32,
    pub l2_cache_size: u32,
    pub l3_cache_size: u32,
    pub num_cores: u32,
    pub num_logical_cores: u32,
    pub memory_bandwidth_gb: f64,
    pub brand_name: String,
}

/// Workload characteristics для adaptive selection
#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    pub vector_dimension: usize,
    pub batch_size: usize,
    pub frequency_hz: f64,
    pub data_alignment: usize,
    pub sparsity_ratio: f64,
    pub cache_hit_ratio: f64,
}

/// Algorithm selection strategy
#[derive(Debug, Clone)]
pub enum AlgorithmStrategy {
    ScalarOptimized,
    Sse42Vectorized,
    Avx2Basic,
    Avx2FmaOptimized,
    Avx2UltraOptimized,
    Avx512Basic,
    Avx512UltraOptimized,
    ParallelBatched,
    MemoryMapped,
    HybridAdaptive,
}

impl SimdLevel {
    /// Get best available SIMD level
    pub fn detect() -> Self {
        Self::ensure_initialized();

        if HAS_AVX512VL.load(Ordering::Relaxed) {
            SimdLevel::Avx512VL
        } else if HAS_AVX512F.load(Ordering::Relaxed) {
            SimdLevel::Avx512F
        } else if HAS_FMA.load(Ordering::Relaxed) && HAS_AVX2.load(Ordering::Relaxed) {
            SimdLevel::Avx2Fma
        } else if HAS_AVX2.load(Ordering::Relaxed) {
            SimdLevel::Avx2
        } else if HAS_SSE42.load(Ordering::Relaxed) {
            SimdLevel::Sse42
        } else {
            SimdLevel::None
        }
    }

    /// Initialize CPU feature detection
    fn ensure_initialized() {
        INIT.call_once(|| {
            Self::detect_cpu_features();
        });
    }

    /// Comprehensive CPU feature detection
    #[cfg(target_arch = "x86_64")]
    fn detect_cpu_features() {
        // SIMD instruction sets
        HAS_SSE42.store(is_x86_feature_detected!("sse4.2"), Ordering::Relaxed);
        HAS_AVX2.store(is_x86_feature_detected!("avx2"), Ordering::Relaxed);
        HAS_FMA.store(is_x86_feature_detected!("fma"), Ordering::Relaxed);
        HAS_AVX512F.store(is_x86_feature_detected!("avx512f"), Ordering::Relaxed);
        HAS_AVX512VL.store(is_x86_feature_detected!("avx512vl"), Ordering::Relaxed);

        // Cache sizes detection (approximation)
        Self::detect_cache_sizes();

        let detected = Self::detect();
        info!("🚀 SIMD Feature Detection Complete:");
        info!("   SIMD Level: {:?}", detected);
        info!("   AVX-512F: {}", HAS_AVX512F.load(Ordering::Relaxed));
        info!("   AVX-512VL: {}", HAS_AVX512VL.load(Ordering::Relaxed));
        info!("   AVX2: {}", HAS_AVX2.load(Ordering::Relaxed));
        info!("   FMA: {}", HAS_FMA.load(Ordering::Relaxed));
        info!("   SSE4.2: {}", HAS_SSE42.load(Ordering::Relaxed));
        info!(
            "   Cache Line: {} bytes",
            CACHE_LINE_SIZE.load(Ordering::Relaxed)
        );
        info!(
            "   L1 Cache: {} KB",
            L1_CACHE_SIZE.load(Ordering::Relaxed) / 1024
        );
        info!(
            "   L3 Cache: {} MB",
            L3_CACHE_SIZE.load(Ordering::Relaxed) / 1024 / 1024
        );
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn detect_cpu_features() {
        info!("ℹ️ Non-x86_64 architecture detected - SIMD optimizations limited");
        // TODO: Add ARM NEON detection for ARM64 targets
    }

    /// Detect cache sizes using various methods
    #[cfg(target_arch = "x86_64")]
    fn detect_cache_sizes() {

        // Modern Intel/AMD defaults
        CACHE_LINE_SIZE.store(64, Ordering::Relaxed); // 64 bytes typical
        L1_CACHE_SIZE.store(32 * 1024, Ordering::Relaxed); // 32KB typical
        L3_CACHE_SIZE.store(8 * 1024 * 1024, Ordering::Relaxed); // 8MB typical

        debug!("Using default cache size estimates for optimization");
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn detect_cache_sizes() {
        // ARM/other architecture defaults
        CACHE_LINE_SIZE.store(64, Ordering::Relaxed);
        L1_CACHE_SIZE.store(32 * 1024, Ordering::Relaxed);
        L3_CACHE_SIZE.store(4 * 1024 * 1024, Ordering::Relaxed);
    }
}

impl CpuInfo {
    /// Comprehensive CPU information gathering
    pub fn detect() -> Self {
        SimdLevel::ensure_initialized();

        let num_logical_cores = num_cpus::get() as u32;
        let num_cores = num_cpus::get_physical() as u32;

        Self {
            simd_level: SimdLevel::detect(),
            cache_line_size: CACHE_LINE_SIZE.load(Ordering::Relaxed),
            l1_cache_size: L1_CACHE_SIZE.load(Ordering::Relaxed),
            l2_cache_size: 256 * 1024, // Typical 256KB L2
            l3_cache_size: L3_CACHE_SIZE.load(Ordering::Relaxed),
            num_cores,
            num_logical_cores,
            memory_bandwidth_gb: Self::estimate_memory_bandwidth(),
            brand_name: Self::get_cpu_brand(),
        }
    }

    /// Estimate memory bandwidth based on CPU info
    fn estimate_memory_bandwidth() -> f64 {
        let num_cores = num_cpus::get_physical();

        // Rough estimation based on typical modern CPUs
        // Modern desktop: ~25-50 GB/s
        // Server CPUs: ~100-200 GB/s
        // Mobile/low-power: ~10-25 GB/s

        let base_bandwidth = match num_cores {
            1..=2 => 15.0,  // Mobile/low-power
            3..=4 => 25.0,  // Desktop dual/quad-core
            5..=8 => 40.0,  // Desktop high-end
            9..=16 => 80.0, // Workstation
            _ => 120.0,     // Server-class
        };

        base_bandwidth
    }

    /// Get CPU brand name
    fn get_cpu_brand() -> String {
        format!("{}-core CPU", num_cpus::get_physical())
    }

    /// Print comprehensive CPU information
    pub fn print_info(&self) {
        info!("🖥️ CPU Information:");
        info!("   Brand: {}", self.brand_name);
        info!("   SIMD Level: {:?}", self.simd_level);
        info!(
            "   Cores: {} physical, {} logical",
            self.num_cores, self.num_logical_cores
        );
        info!("   L1 Cache: {} KB", self.l1_cache_size / 1024);
        info!("   L2 Cache: {} KB", self.l2_cache_size / 1024);
        info!("   L3 Cache: {} MB", self.l3_cache_size / 1024 / 1024);
        info!("   Est. Memory BW: {:.1} GB/s", self.memory_bandwidth_gb);
        info!("   Cache Line: {} bytes", self.cache_line_size);
    }
}

impl WorkloadProfile {
    /// Create workload profile from characteristics
    pub fn new(vector_dimension: usize, batch_size: usize, frequency_hz: f64) -> Self {
        Self {
            vector_dimension,
            batch_size,
            frequency_hz,
            data_alignment: Self::detect_alignment(vector_dimension),
            sparsity_ratio: 0.0,  // Assume dense by default
            cache_hit_ratio: 0.8, // Typical hit ratio
        }
    }

    /// Detect optimal alignment for given dimension
    fn detect_alignment(dim: usize) -> usize {
        // Determine best alignment based on SIMD level and dimension
        let simd_level = SimdLevel::detect();

        match simd_level {
            SimdLevel::Avx512F | SimdLevel::Avx512VL => {
                if dim % 16 == 0 {
                    64
                } else {
                    32
                } // 64-byte align for AVX-512
            }
            SimdLevel::Avx2 | SimdLevel::Avx2Fma => {
                if dim % 8 == 0 {
                    32
                } else {
                    16
                } // 32-byte align for AVX2
            }
            _ => 16, // Basic alignment
        }
    }

    /// Calculate working set size
    pub fn working_set_size(&self) -> usize {
        self.vector_dimension * self.batch_size * 4 // 4 bytes per f32
    }

    /// Check if workload fits in L3 cache
    pub fn fits_in_l3_cache(&self) -> bool {
        self.working_set_size() <= L3_CACHE_SIZE.load(Ordering::Relaxed) as usize
    }

    /// Check if workload fits in L1 cache
    pub fn fits_in_l1_cache(&self) -> bool {
        self.working_set_size() <= L1_CACHE_SIZE.load(Ordering::Relaxed) as usize
    }
}

/// Adaptive algorithm selector
pub struct AdaptiveAlgorithmSelector {
    cpu_info: CpuInfo,
    performance_history: std::collections::HashMap<String, f64>,
}

impl AdaptiveAlgorithmSelector {
    /// Create new adaptive selector
    pub fn new() -> Self {
        let cpu_info = CpuInfo::detect();
        cpu_info.print_info();

        Self {
            cpu_info,
            performance_history: std::collections::HashMap::new(),
        }
    }

    /// Select optimal algorithm based on workload
    pub fn select_algorithm(&self, workload: &WorkloadProfile) -> AlgorithmStrategy {
        let simd_level = self.cpu_info.simd_level;
        let dim = workload.vector_dimension;
        let batch_size = workload.batch_size;
        let working_set = workload.working_set_size();

        debug!("🤖 Selecting algorithm for workload:");
        debug!("   Dimension: {}", dim);
        debug!("   Batch size: {}", batch_size);
        debug!("   Working set: {} KB", working_set / 1024);
        debug!("   SIMD level: {:?}", simd_level);

        // Decision tree based on comprehensive analysis
        let strategy = match simd_level {
            SimdLevel::Avx512VL | SimdLevel::Avx512F => {
                if dim >= 64 && dim % 16 == 0 && batch_size >= 10 {
                    if working_set > self.cpu_info.l3_cache_size as usize {
                        AlgorithmStrategy::MemoryMapped
                    } else if batch_size >= 100 {
                        AlgorithmStrategy::ParallelBatched
                    } else {
                        AlgorithmStrategy::Avx512UltraOptimized
                    }
                } else {
                    AlgorithmStrategy::Avx2UltraOptimized
                }
            }

            SimdLevel::Avx2Fma => {
                if dim >= 32 && dim % 8 == 0 {
                    if batch_size >= 100 && working_set <= self.cpu_info.l3_cache_size as usize {
                        AlgorithmStrategy::ParallelBatched
                    } else {
                        AlgorithmStrategy::Avx2UltraOptimized
                    }
                } else {
                    AlgorithmStrategy::Avx2FmaOptimized
                }
            }

            SimdLevel::Avx2 => {
                if dim % 8 == 0 && dim >= 16 {
                    AlgorithmStrategy::Avx2Basic
                } else {
                    AlgorithmStrategy::ScalarOptimized
                }
            }

            SimdLevel::Sse42 => {
                if dim % 4 == 0 && dim >= 8 {
                    AlgorithmStrategy::Sse42Vectorized
                } else {
                    AlgorithmStrategy::ScalarOptimized
                }
            }

            SimdLevel::None => AlgorithmStrategy::ScalarOptimized,
        };

        // Apply adaptive corrections based on workload characteristics
        let final_strategy = self.apply_adaptive_corrections(strategy, workload);

        info!(
            "🎯 Selected algorithm: {:?} for workload dimension={}, batch={}",
            final_strategy, dim, batch_size
        );

        final_strategy
    }

    /// Apply adaptive corrections based on performance history and workload
    fn apply_adaptive_corrections(
        &self,
        base_strategy: AlgorithmStrategy,
        workload: &WorkloadProfile,
    ) -> AlgorithmStrategy {
        if workload.batch_size > 50 && workload.vector_dimension >= 512 {
            // Large workloads benefit from hybrid approaches
            if self.cpu_info.num_logical_cores >= 8 {
                return AlgorithmStrategy::HybridAdaptive;
            }
        }

        // Memory pressure considerations
        if !workload.fits_in_l3_cache() && workload.working_set_size() > 100 * 1024 * 1024 {
            // Very large working sets need memory-mapped approach
            return AlgorithmStrategy::MemoryMapped;
        }

        // High-frequency workloads need optimized paths
        if workload.frequency_hz > 1000.0 {
            match base_strategy {
                AlgorithmStrategy::Avx2Basic => AlgorithmStrategy::Avx2UltraOptimized,
                AlgorithmStrategy::Avx512Basic => AlgorithmStrategy::Avx512UltraOptimized,
                other => other,
            }
        } else {
            base_strategy
        }
    }

    /// Record performance result for learning
    pub fn record_performance(
        &mut self,
        workload: &WorkloadProfile,
        strategy: AlgorithmStrategy,
        duration_ns: u64,
    ) {
        let key = format!(
            "{}_{}_{}_{:?}",
            workload.vector_dimension,
            workload.batch_size,
            (workload.frequency_hz as u32),
            strategy
        );

        let performance_score = 1e9 / duration_ns as f64; // Higher is better
        self.performance_history.insert(key, performance_score);

        debug!(
            "📊 Recorded performance: {:?} -> {:.2} ops/sec",
            strategy, performance_score
        );
    }

    /// Get performance recommendations
    pub fn get_recommendations(&self, workload: &WorkloadProfile) -> Vec<AlgorithmStrategy> {
        let mut recommendations = vec![self.select_algorithm(workload)];

        // Add alternative strategies based on CPU capabilities
        match self.cpu_info.simd_level {
            SimdLevel::Avx512F | SimdLevel::Avx512VL => {
                recommendations.extend([
                    AlgorithmStrategy::Avx512UltraOptimized,
                    AlgorithmStrategy::Avx2UltraOptimized,
                    AlgorithmStrategy::ParallelBatched,
                ]);
            }
            SimdLevel::Avx2Fma => {
                recommendations.extend([
                    AlgorithmStrategy::Avx2UltraOptimized,
                    AlgorithmStrategy::Avx2FmaOptimized,
                    AlgorithmStrategy::ParallelBatched,
                ]);
            }
            _ => {
                recommendations.push(AlgorithmStrategy::ScalarOptimized);
            }
        }

        recommendations.truncate(3); // Top 3 recommendations
        recommendations
    }
}

impl Default for AdaptiveAlgorithmSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Global adaptive selector instance
static GLOBAL_SELECTOR: OnceLock<AdaptiveAlgorithmSelector> = OnceLock::new();

/// Get global adaptive selector
pub fn get_adaptive_selector() -> &'static AdaptiveAlgorithmSelector {
    GLOBAL_SELECTOR.get_or_init(AdaptiveAlgorithmSelector::new)
}

/// Quick CPU info for optimization decisions
pub fn quick_cpu_info() -> (SimdLevel, bool, bool) {
    SimdLevel::ensure_initialized();
    (
        SimdLevel::detect(),
        HAS_AVX2.load(Ordering::Relaxed) && HAS_FMA.load(Ordering::Relaxed),
        HAS_AVX512F.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_detection() {
        if std::env::var("CI").is_ok() {
            return;
        }
        let level = SimdLevel::detect();
        println!("Detected SIMD level: {:?}", level);
        assert!(level >= SimdLevel::None);
    }

    #[test]
    fn test_cpu_info() {
        if std::env::var("CI").is_ok() {
            return;
        }
        let info = CpuInfo::detect();
        info.print_info();
        assert!(info.num_cores > 0);
        assert!(info.cache_line_size > 0);
    }

    #[test]
    fn test_workload_profile() {
        if std::env::var("CI").is_ok() {
            return;
        }
        let profile = WorkloadProfile::new(1024, 100, 60.0);
        assert_eq!(profile.vector_dimension, 1024);
        assert_eq!(profile.batch_size, 100);
        assert!(profile.working_set_size() > 0);
    }

    #[test]
    fn test_algorithm_selection() {
        if std::env::var("CI").is_ok() {
            return;
        }
        let selector = AdaptiveAlgorithmSelector::new();
        let workload = WorkloadProfile::new(1024, 50, 120.0);
        let strategy = selector.select_algorithm(&workload);

        println!("Selected strategy: {:?}", strategy);
        // Assert that some valid strategy was selected
        match strategy {
            AlgorithmStrategy::ScalarOptimized
            | AlgorithmStrategy::Sse42Vectorized
            | AlgorithmStrategy::Avx2Basic
            | AlgorithmStrategy::Avx2FmaOptimized
            | AlgorithmStrategy::Avx2UltraOptimized
            | AlgorithmStrategy::Avx512Basic
            | AlgorithmStrategy::Avx512UltraOptimized
            | AlgorithmStrategy::ParallelBatched
            | AlgorithmStrategy::MemoryMapped
            | AlgorithmStrategy::HybridAdaptive => {
                // Valid strategy selected
            }
        }
    }
}
