use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[cfg(target_os = "windows")]
const LIB_NAME: &str = "onnxruntime.dll";
#[cfg(target_os = "linux")]
const LIB_NAME: &str = "libonnxruntime.so";
#[cfg(target_os = "macos")]
const LIB_NAME: &str = "libonnxruntime.dylib";

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Explicit env override
    if let Ok(p) = std::env::var("ORT_DYLIB_PATH") {
        paths.push(PathBuf::from(p));
    }
    if let Ok(dir) = std::env::var("ORT_DIR") {
        paths.push(Path::new(&dir).join("lib").join(LIB_NAME));
        paths.push(Path::new(&dir).join(LIB_NAME));
    }

    // Repo-local script install location
    paths.push(Path::new("scripts/onnxruntime/lib").join(LIB_NAME));
    paths.push(Path::new("./scripts/onnxruntime/lib").join(LIB_NAME));
    paths.push(Path::new("../scripts/onnxruntime/lib").join(LIB_NAME));
    paths.push(Path::new("../../scripts/onnxruntime/lib").join(LIB_NAME));

    // Common system locations
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        paths.push(Path::new("/usr/local/lib").join(LIB_NAME));
        paths.push(Path::new("/usr/lib").join(LIB_NAME));
        paths.push(Path::new("/opt/onnxruntime/lib").join(LIB_NAME));
    }

    // Near current executable
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe.clone();
        for _ in 0..4 {
            if let Some(dir) = p.parent() {
                paths.push(dir.join(LIB_NAME));
                paths.push(dir.join("onnxruntime").join("lib").join(LIB_NAME));
                p = dir.to_path_buf();
            }
        }
    }

    paths
}

pub fn configure_ort_env() {
    // If already set, keep
    if std::env::var("ORT_DYLIB_PATH").is_ok() {
        return;
    }

    for path in candidate_paths() {
        if path.exists() {
            if let Some(_parent) = path.parent() {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    let key = if cfg!(target_os = "macos") {
                        "DYLD_LIBRARY_PATH"
                    } else {
                        "LD_LIBRARY_PATH"
                    };
                    let mut new_val = parent.display().to_string();
                    if let Ok(prev) = std::env::var(key) {
                        if !prev.is_empty() {
                            new_val = format!("{}:{}", parent.display(), prev);
                        }
                    }
                    std::env::set_var(key, &new_val);
                }

                std::env::set_var("ORT_DYLIB_PATH", &path);
                info!(target: "ai::ort_setup", "ONNX Runtime set: ORT_DYLIB_PATH={} ", path.display());
            } else {
                warn!(target: "ai::ort_setup", "Found ORT library but no parent dir: {}", path.display());
            }
            return;
        }
    }
    warn!(target: "ai::ort_setup", "ONNX Runtime library not found. If you see init errors, run scripts/install_onnxruntime.sh or set ORT_DYLIB_PATH.");
}
