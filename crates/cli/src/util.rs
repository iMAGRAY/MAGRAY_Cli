use std::path::PathBuf;

pub fn magray_home() -> PathBuf {
    if let Ok(custom) = std::env::var("MAGRAY_HOME") {
        let p = PathBuf::from(custom);
        std::fs::create_dir_all(&p).ok();
        return p;
    }
    let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push(".magray");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn default_tasks_db_path() -> PathBuf {
    let mut dir = magray_home();
    dir.push("tasks.db");
    dir
}

#[allow(dead_code)]
pub fn artifacts_dir() -> PathBuf {
    let mut dir = magray_home();
    dir.push("artifacts");
    std::fs::create_dir_all(&dir).ok();
    dir
}