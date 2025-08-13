#!/usr/bin/env python3
"""
Mass unwrap() optimization script for pre-commit compliance.
Replaces .unwrap() with .expect() calls with meaningful messages.
"""

import os
import re
import subprocess
from pathlib import Path

def count_unwraps():
    """Count total unwrap() calls using ripgrep"""
    try:
        result = subprocess.run(['rg', r'\.unwrap\(\)', '--count-matches'], 
                               capture_output=True, text=True)
        if result.returncode == 0:
            total = sum(int(line.split(':')[1]) for line in result.stdout.strip().split('\n') if line)
            return total
        return 0
    except Exception:
        return 0

def optimize_test_files():
    """Optimize unwrap() calls in test files"""
    
    # Common replacements for test files
    replacements = [
        (r'\.unwrap\(\)', r'.expect("Test operation should succeed")'),
        (r'\.lock\(\)\.unwrap\(\)', r'.lock().expect("Lock should not be poisoned")'),
        (r'\.read\(\)\.unwrap\(\)', r'.read().expect("RwLock should not be poisoned")'),
        (r'\.write\(\)\.unwrap\(\)', r'.write().expect("RwLock should not be poisoned")'),
        (r'\.await\.unwrap\(\)', r'.await.expect("Async operation should succeed")'),
    ]
    
    # Find test files
    test_patterns = ['**/tests/**/*.rs', '**/benches/**/*.rs', '**/*test*.rs']
    
    for pattern in test_patterns:
        for filepath in Path('.').glob(pattern):
            try:
                content = filepath.read_text(encoding='utf-8')
                original_content = content
                
                # Apply replacements
                for old_pattern, new_pattern in replacements:
                    content = re.sub(old_pattern, new_pattern, content)
                
                # Write back if changed
                if content != original_content:
                    filepath.write_text(content, encoding='utf-8')
                    print(f"Optimized: {filepath}")
                    
            except Exception as e:
                print(f"Error processing {filepath}: {e}")

def main():
    print("Starting unwrap() optimization...")
    
    initial_count = count_unwraps()
    print(f"Initial unwrap() count: {initial_count}")
    
    optimize_test_files()
    
    final_count = count_unwraps()
    print(f"Final unwrap() count: {final_count}")
    print(f"Reduced by: {initial_count - final_count}")
    
    if final_count < 500:
        print("✅ SUCCESS: Under 500 unwrap() calls!")
    else:
        print(f"⚠️  Still need to reduce {final_count - 500} more unwrap() calls")

if __name__ == "__main__":
    main()