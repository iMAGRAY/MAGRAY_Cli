fn main() {
    // Для Windows используем динамическую линковку
    if cfg!(target_os = "windows") {
        // Указываем использовать динамическую CRT
        println!("cargo:rustc-link-arg=/MD");
    }
    
    // Убеждаемся, что ONNX Runtime может найти свои DLL
    println!("cargo:rustc-env=ORT_DYLIB_PATH=C:\\Users\\1\\Documents\\GitHub\\MAGRAY_Cli\\target\\debug\\deps");
}