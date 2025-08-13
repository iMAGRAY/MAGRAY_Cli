/// Helper script to create test WASM modules for sandbox testing
/// This generates the binary WASM files used in the test suite

use std::fs;
use std::path::Path;

fn main() {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets");
    
    // Create safe add module
    let safe_add_wasm = create_safe_add_module();
    fs::write(assets_dir.join("safe_add.wasm"), safe_add_wasm)
        .expect("Failed to write safe_add.wasm");
    
    // Create malicious filesystem module
    let malicious_fs_wasm = create_malicious_fs_module();
    fs::write(assets_dir.join("malicious_fs.wasm"), malicious_fs_wasm)
        .expect("Failed to write malicious_fs.wasm");
    
    // Create malicious network module
    let malicious_net_wasm = create_malicious_net_module();
    fs::write(assets_dir.join("malicious_net.wasm"), malicious_net_wasm)
        .expect("Failed to write malicious_net.wasm");
    
    // Create resource bomb module
    let resource_bomb_wasm = create_resource_bomb_module();
    fs::write(assets_dir.join("resource_bomb.wasm"), resource_bomb_wasm)
        .expect("Failed to write resource_bomb.wasm");
    
    println!("Test WASM modules created successfully!");
}

fn create_safe_add_module() -> Vec<u8> {
    // Safe WASM module with add(i32, i32) -> i32 function
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

fn create_malicious_fs_module() -> Vec<u8> {
    // Malicious WASM module attempting filesystem access
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        0x01, 0x05, // Type section
        0x01, // 1 type
        0x60, 0x00, 0x01, 0x7f, // func type: () -> i32
        0x02, 0x18, // Import section - WASI functions
        0x01, // 1 import
        0x04, 0x77, 0x61, 0x73, 0x69, // "wasi"
        0x0a, 0x70, 0x61, 0x74, 0x68, 0x5f, 0x6f, 0x70, 0x65, 0x6e, // "path_open"
        0x00, 0x00, // function import, type 0
        0x03, 0x02, // Function section
        0x01, // 1 function
        0x00, // function 0, type 0
        0x07, 0x0b, // Export section
        0x01, // 1 export
        0x06, 0x65, 0x73, 0x63, 0x61, 0x70, 0x65, // "escape"
        0x00, 0x01, // function export, function 1
        0x0a, 0x05, // Code section
        0x01, // 1 function body
        0x03, // body size
        0x00, // 0 locals
        0x41, 0x42, // i32.const 66 (return value)
        0x0b, // end
    ]
}

fn create_malicious_net_module() -> Vec<u8> {
    // Malicious WASM module attempting network access
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        0x01, 0x05, // Type section
        0x01, // 1 type
        0x60, 0x00, 0x01, 0x7f, // func type: () -> i32
        0x02, 0x18, // Import section - WASI socket functions
        0x01, // 1 import
        0x04, 0x77, 0x61, 0x73, 0x69, // "wasi"
        0x0a, 0x73, 0x6f, 0x63, 0x6b, 0x5f, 0x6f, 0x70, 0x65, 0x6e, // "sock_open"
        0x00, 0x00, // function import, type 0
        0x03, 0x02, // Function section
        0x01, // 1 function
        0x00, // function 0, type 0
        0x07, 0x0b, // Export section
        0x01, // 1 export
        0x07, 0x63, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, // "connect"
        0x00, 0x01, // function export, function 1
        0x0a, 0x05, // Code section
        0x01, // 1 function body
        0x03, // body size
        0x00, // 0 locals
        0x41, 0x43, // i32.const 67 (return value)
        0x0b, // end
    ]
}

fn create_resource_bomb_module() -> Vec<u8> {
    // WASM module that attempts to exhaust resources
    vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        0x01, 0x05, // Type section
        0x01, // 1 type
        0x60, 0x00, 0x01, 0x7f, // func type: () -> i32
        0x03, 0x02, // Function section
        0x01, // 1 function
        0x00, // function 0, type 0
        0x05, 0x03, // Memory section
        0x01, // 1 memory
        0x00, 0x01, // initial: 1 page (64KB), no maximum
        0x07, 0x08, // Export section
        0x01, // 1 export
        0x04, 0x62, 0x6f, 0x6d, 0x62, // "bomb"
        0x00, 0x00, // function export, function 0
        0x0a, 0x08, // Code section
        0x01, // 1 function body
        0x06, // body size
        0x00, // 0 locals
        0x03, 0x40, // loop
        0x0c, 0x00, // br 0 (infinite loop)
        0x0b, // end loop
        0x41, 0x44, // i32.const 68 (unreachable)
        0x0b, // end
    ]
}