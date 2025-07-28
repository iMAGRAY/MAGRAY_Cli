-- SQLite schema

-- Short/Medium term KV/table example
CREATE TABLE IF NOT EXISTS kv_short (
  key TEXT PRIMARY KEY,
  value BLOB,
  meta  TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Example structured table for MediumTerm
CREATE TABLE IF NOT EXISTS facts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT,
  title TEXT,
  body TEXT,
  meta  TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- TaskBoard
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  desc TEXT,
  state TEXT NOT NULL,
  priority INTEGER,
  created_at TEXT,
  due_at TEXT,
  last_touch TEXT,
  staleness REAL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS task_deps (
  task_id TEXT,
  dep_id  TEXT,
  PRIMARY KEY(task_id, dep_id)
);

CREATE TABLE IF NOT EXISTS task_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT,
  ts TEXT,
  event TEXT,
  payload TEXT
);

-- Tool specs
CREATE TABLE IF NOT EXISTS tool_specs (
  id TEXT PRIMARY KEY,
  name TEXT,
  desc TEXT,
  io_schema TEXT,
  tags TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Embedding cache
CREATE TABLE IF NOT EXISTS embed_cache (
  hash TEXT PRIMARY KEY,
  model TEXT,
  vec   BLOB,
  ts    TEXT DEFAULT CURRENT_TIMESTAMP
);
