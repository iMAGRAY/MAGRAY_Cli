"""
Utility Functions

Common utilities for CTL sync operations.
"""

import sys
from pathlib import Path
from typing import Dict, Any

try:
    from colorama import init, Fore, Style
    init(autoreset=True)
    COLORS_AVAILABLE = True
except ImportError:
    COLORS_AVAILABLE = False


def colored_print(text: str, color: str = "white", bold: bool = False) -> None:
    """Print colored text if colorama is available"""
    if not COLORS_AVAILABLE:
        print(text)
        return
    
    color_map = {
        "red": Fore.RED,
        "green": Fore.GREEN,
        "yellow": Fore.YELLOW,
        "blue": Fore.BLUE,
        "magenta": Fore.MAGENTA,
        "cyan": Fore.CYAN,
        "white": Fore.WHITE,
    }
    
    color_code = color_map.get(color.lower(), Fore.WHITE)
    style = Style.BRIGHT if bold else ""
    
    print(f"{style}{color_code}{text}")


def print_stats(stats: Dict[str, Any]) -> None:
    """Print synchronization statistics in a formatted way"""
    colored_print("\nCTL Sync Statistics", "cyan", bold=True)
    colored_print("=" * 50, "cyan")
    
    print(f"Total Components: {stats['total_components']}")
    print(f"CTL v2.0 Format: {stats['ctl2_components']}")
    print(f"CTL v3.0 Format: {stats['ctl3_components']}")
    
    if stats['kind_distribution']:
        colored_print("\nComponent Distribution:", "yellow", bold=True)
        for kind, count in sorted(stats['kind_distribution'].items()):
            kind_name = get_kind_name(kind)
            print(f"  {kind} ({kind_name}): {count}")
    
    print(f"\nLast Scan: {stats['last_scan']}")


def get_kind_name(kind: str) -> str:
    """Get full name for component kind code"""
    kind_names = {
        'T': 'Test',
        'A': 'Agent',
        'B': 'Batch',
        'F': 'Function',
        'M': 'Module',
        'S': 'Service',
        'R': 'Resource',
        'P': 'Process',
        'D': 'Data',
        'C': 'Component',
        'E': 'Error'
    }
    return kind_names.get(kind, 'Unknown')


def find_project_root(start_path: Path = None) -> Path:
    """
    Find project root by looking for characteristic files
    
    Args:
        start_path: Starting directory (default: current directory)
        
    Returns:
        Project root path
    """
    if start_path is None:
        start_path = Path.cwd()
    
    current = start_path.resolve()
    
    # Look for project indicators
    indicators = ['Cargo.toml', 'CLAUDE.md', '.git', 'crates']
    
    while current != current.parent:
        if any((current / indicator).exists() for indicator in indicators):
            return current
        current = current.parent
    
    # If not found, return starting path
    return start_path


def validate_project_structure(project_root: Path) -> tuple[bool, list[str]]:
    """
    Validate that project has expected structure
    
    Args:
        project_root: Project root directory
        
    Returns:
        (is_valid, error_messages)
    """
    errors = []
    
    # Check for required directories/files
    required_paths = {
        'crates': 'Crates directory not found',
        'CLAUDE.md': 'CLAUDE.md file not found'
    }
    
    for path_name, error_msg in required_paths.items():
        path = project_root / path_name
        if not path.exists():
            errors.append(error_msg)
    
    return len(errors) == 0, errors


def setup_logging() -> None:
    """Setup basic logging configuration"""
    import logging
    
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.StreamHandler(sys.stdout)
        ]
    )


def print_banner() -> None:
    """Print application banner"""
    banner = """
====================================================
         CTL v3.0 Tensor Sync Daemon (Python)     
                                                   
  Fast & Flexible - Rapid CTL Language Adaptation 
====================================================
    """
    colored_print(banner, "cyan", bold=True)