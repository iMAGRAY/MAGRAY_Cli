#!/usr/bin/env python3
"""
Найти все файлы с MAGRAY в содержимом
"""

from pathlib import Path
import shutil

def find_magray_files():
    """Найти файлы с MAGRAY CLI"""
    
    history_base = Path(r'C:\Users\1\AppData\Roaming\Cursor\User\History')
    magray_files = []
    
    print("Searching for MAGRAY files...")
    print("=" * 80)
    
    count = 0
    for history_dir in history_base.iterdir():
        if not history_dir.is_dir():
            continue
            
        for file_path in history_dir.glob('*'):
            if file_path.is_file() and file_path.suffix in ['.md', '.toml', '.rs']:
                try:
                    content = file_path.read_text(encoding='utf-8', errors='ignore')
                    if 'MAGRAY' in content and 'CLI' in content:
                        size = file_path.stat().st_size
                        # Ищем первую строку с MAGRAY
                        lines = content.split('\n')
                        magray_line = None
                        for line in lines[:10]:
                            if 'MAGRAY' in line:
                                magray_line = line[:100]
                                break
                        
                        magray_files.append({
                            'path': file_path,
                            'size': size,
                            'line': magray_line or 'MAGRAY found',
                            'dir': history_dir.name
                        })
                        count += 1
                        
                        if count <= 20:
                            print(f"Found: {file_path.name}")
                            print(f"  Dir: {history_dir.name}")
                            print(f"  Size: {size:,} bytes")
                            print(f"  Line: {magray_line or 'MAGRAY found'}")
                            print()
                            
                except:
                    pass
    
    print(f"\nTotal found: {len(magray_files)} files with MAGRAY")
    
    # Найдем самый большой README с MAGRAY
    readme_files = [f for f in magray_files if f['path'].suffix == '.md' and f['size'] > 2000 and f['size'] < 10000]
    if readme_files:
        readme_files.sort(key=lambda x: x['size'], reverse=True)
        best_readme = readme_files[0]
        
        print("\n" + "=" * 80)
        print("BEST README CANDIDATE:")
        print(f"  File: {best_readme['path']}")
        print(f"  Size: {best_readme['size']:,} bytes")
        print(f"  Content: {best_readme['line']}")
        
        # Восстанавливаем
        target = Path(r'C:\Users\1\Documents\GitHub\MAGRAY_Cli\README.md')
        try:
            shutil.copy2(best_readme['path'], target)
            print(f"\nRestored to: {target}")
            
            # Показываем первые строки
            content = target.read_text(encoding='utf-8', errors='ignore')
            lines = content.split('\n')[:5]
            print("\nFirst 5 lines:")
            for line in lines:
                # Убираем эмодзи для вывода
                clean_line = ''.join(c for c in line if ord(c) < 65536)
                print(f"  {clean_line}")
                
        except Exception as e:
            print(f"Error: {e}")
    
    return magray_files

if __name__ == "__main__":
    find_magray_files()