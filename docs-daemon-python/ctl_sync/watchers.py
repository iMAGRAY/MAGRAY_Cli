"""
File System Watchers

Advanced file monitoring with intelligent debouncing and filtering.
"""

import time
from pathlib import Path
from typing import Callable, Optional

from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler, FileModifiedEvent, FileCreatedEvent


class CtlFileHandler(FileSystemEventHandler):
    """File system event handler for CTL sync"""
    
    def __init__(self, callback: Callable[[], None], debounce_seconds: float = 0.5):
        self.callback = callback
        self.debounce_seconds = debounce_seconds
        self.last_trigger = 0.0
        
    def should_process_file(self, file_path: str) -> bool:
        """Check if file should trigger sync"""
        path = Path(file_path)
        
        # Only process Rust files
        if path.suffix != '.rs':
            return False
        
        # Skip target directory
        if 'target' in path.parts:
            return False
        
        # Skip temporary files
        if path.name.startswith('.'):
            return False
        
        return True
    
    def trigger_sync(self) -> None:
        """Trigger sync with debouncing"""
        current_time = time.time()
        
        # Debounce rapid file changes
        if current_time - self.last_trigger < self.debounce_seconds:
            return
        
        self.last_trigger = current_time
        
        # Wait a bit more to catch rapid successive changes
        time.sleep(self.debounce_seconds)
        
        # Check if another event occurred during the wait
        if time.time() - self.last_trigger < self.debounce_seconds:
            return
        
        print("File change detected, triggering sync...")
        self.callback()
    
    def on_modified(self, event):
        """Handle file modification events"""
        if not isinstance(event, FileModifiedEvent) or event.is_directory:
            return
        
        if self.should_process_file(event.src_path):
            self.trigger_sync()
    
    def on_created(self, event):
        """Handle file creation events"""
        if not isinstance(event, FileCreatedEvent) or event.is_directory:
            return
        
        if self.should_process_file(event.src_path):
            self.trigger_sync()


class CtlWatcher:
    """File system watcher for CTL synchronization"""
    
    def __init__(self, watch_path: Path, sync_callback: Callable[[], None]):
        self.watch_path = watch_path
        self.sync_callback = sync_callback
        self.observer: Optional[Observer] = None
        self.handler = CtlFileHandler(sync_callback)
    
    def start(self) -> None:
        """Start file system monitoring"""
        if not self.watch_path.exists():
            print(f"Warning: Watch path does not exist: {self.watch_path}")
            return
        
        self.observer = Observer()
        self.observer.schedule(
            self.handler,
            str(self.watch_path),
            recursive=True
        )
        
        self.observer.start()
        print(f"Watching for changes in: {self.watch_path}")
        print("Press Ctrl+C to stop...")
        
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            self.stop()
    
    def stop(self) -> None:
        """Stop file system monitoring"""
        if self.observer:
            self.observer.stop()
            self.observer.join()
            print("Stopped watching for changes")