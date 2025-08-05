"""
CTL Sync Main Entry Point

Command-line interface for the CTL synchronization daemon.
"""

import sys
from pathlib import Path
from typing import Optional

import click

from .core import CtlSync
from .watchers import CtlWatcher
from .utils import colored_print, print_stats, print_banner, find_project_root, validate_project_structure


@click.command()
@click.argument('mode', default='once', type=click.Choice(['once', 'watch', 'stats']))
@click.option('--project-root', '-p', type=click.Path(exists=True, path_type=Path),
              help='Project root directory (auto-detected if not specified)')
@click.option('--verbose', '-v', is_flag=True, help='Verbose output')
@click.option('--no-color', is_flag=True, help='Disable colored output')
def main(mode: str, project_root: Optional[Path], verbose: bool, no_color: bool):
    """
    CTL v3.0 Tensor Sync Daemon (Python)
    
    Synchronizes CTL annotations from Rust code to CLAUDE.md with support
    for both CTL v2.0 JSON and CTL v3.0 Tensor formats.
    
    MODES:
    \b
    - once: Perform one-time synchronization (default)
    - watch: Watch for file changes and sync continuously  
    - stats: Show current synchronization statistics
    """
    
    if not no_color:
        print_banner()
    
    # Auto-detect project root if not specified
    if project_root is None:
        project_root = find_project_root()
        if verbose:
            colored_print(f"Auto-detected project root: {project_root}", "blue")
    
    # Validate project structure
    is_valid, errors = validate_project_structure(project_root)
    if not is_valid:
        colored_print("Project validation failed:", "red", bold=True)
        for error in errors:
            colored_print(f"  - {error}", "red")
        sys.exit(1)
    
    # Initialize CTL sync
    try:
        ctl_sync = CtlSync(project_root)
    except Exception as e:
        colored_print(f"Failed to initialize CTL sync: {e}", "red", bold=True)
        sys.exit(1)
    
    # Execute requested mode
    try:
        if mode == 'once':
            execute_once(ctl_sync, verbose)
        elif mode == 'watch':
            execute_watch(ctl_sync, verbose)
        elif mode == 'stats':
            execute_stats(ctl_sync)
    except KeyboardInterrupt:
        colored_print("\nInterrupted by user", "yellow")
        sys.exit(0)
    except Exception as e:
        colored_print(f"Error during execution: {e}", "red", bold=True)
        if verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


def execute_once(ctl_sync: CtlSync, verbose: bool) -> None:
    """Execute one-time synchronization"""
    colored_print("Starting one-time sync...", "blue", bold=True)
    
    try:
        ctl_sync.sync_once()
        colored_print("Synchronization complete!", "green", bold=True)
        
        if verbose:
            stats = ctl_sync.get_stats()
            print_stats(stats)
            
    except Exception as e:
        colored_print(f"Sync failed: {e}", "red", bold=True)
        raise


def execute_watch(ctl_sync: CtlSync, verbose: bool) -> None:
    """Execute continuous file watching"""
    colored_print("Starting watch mode...", "blue", bold=True)
    
    # Perform initial sync
    ctl_sync.sync_once()
    
    # Start file watcher
    watcher = CtlWatcher(ctl_sync.crates_path, ctl_sync.sync_once)
    watcher.start()


def execute_stats(ctl_sync: CtlSync) -> None:
    """Show synchronization statistics"""
    colored_print("Gathering statistics...", "blue", bold=True)
    
    stats = ctl_sync.get_stats()
    print_stats(stats)


if __name__ == '__main__':
    main()