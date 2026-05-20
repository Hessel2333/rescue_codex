CREATE TABLE IF NOT EXISTS data_sources (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  label TEXT NOT NULL,
  root_path TEXT NOT NULL,
  config_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_data_sources_kind_root
  ON data_sources(kind, root_path);

CREATE TABLE IF NOT EXISTS imports (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  parser_key TEXT NOT NULL,
  parser_version TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  files_total INTEGER NOT NULL DEFAULT 0,
  files_success INTEGER NOT NULL DEFAULT 0,
  files_failed INTEGER NOT NULL DEFAULT 0,
  warnings_count INTEGER NOT NULL DEFAULT 0,
  errors_count INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(source_id) REFERENCES data_sources(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_imports_source_id ON imports(source_id);
CREATE INDEX IF NOT EXISTS idx_imports_started_at ON imports(started_at DESC);

CREATE TABLE IF NOT EXISTS source_files (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  rel_path TEXT,
  file_ext TEXT NOT NULL,
  size_bytes INTEGER NOT NULL DEFAULT 0,
  mtime_ms INTEGER NOT NULL DEFAULT 0,
  sha256 TEXT NOT NULL,
  status TEXT NOT NULL,
  last_import_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(abs_path, sha256),
  FOREIGN KEY(source_id) REFERENCES data_sources(id) ON DELETE CASCADE,
  FOREIGN KEY(last_import_id) REFERENCES imports(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_source_files_source_id ON source_files(source_id);

CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  source_file_id TEXT,
  thread_title TEXT,
  cwd TEXT,
  originator TEXT,
  source TEXT,
  model_provider TEXT,
  cli_version TEXT,
  started_at TEXT,
  updated_at TEXT,
  first_user_message TEXT,
  raw_event_count INTEGER NOT NULL DEFAULT 0,
  user_message_count INTEGER NOT NULL DEFAULT 0,
  assistant_message_count INTEGER NOT NULL DEFAULT 0,
  tool_call_count INTEGER NOT NULL DEFAULT 0,
  turn_count INTEGER NOT NULL DEFAULT 0,
  duration_sec INTEGER NOT NULL DEFAULT 0,
  warning_count INTEGER NOT NULL DEFAULT 0,
  warnings_json TEXT NOT NULL DEFAULT '[]',
  FOREIGN KEY(source_file_id) REFERENCES source_files(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_cwd ON sessions(cwd);
CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);

CREATE TABLE IF NOT EXISTS session_events_raw (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  ts TEXT,
  outer_type TEXT NOT NULL,
  inner_type TEXT,
  payload_json TEXT NOT NULL,
  warning_code TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_events_lookup
  ON session_events_raw(session_id, outer_type, inner_type);

CREATE INDEX IF NOT EXISTS idx_session_events_session_seq
  ON session_events_raw(session_id, seq);

CREATE TABLE IF NOT EXISTS session_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  turn_id TEXT,
  role TEXT,
  kind TEXT NOT NULL,
  text TEXT,
  ts TEXT,
  tool_name TEXT,
  phase TEXT,
  meta_json TEXT NOT NULL DEFAULT '{}',
  text_hash TEXT NOT NULL DEFAULT '',
  FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_messages_lookup
  ON session_messages(session_id, ts);

CREATE TABLE IF NOT EXISTS import_issues (
  id TEXT PRIMARY KEY,
  import_id TEXT NOT NULL,
  source_file_id TEXT,
  severity TEXT NOT NULL,
  code TEXT NOT NULL,
  message TEXT NOT NULL,
  line_no INTEGER,
  raw_excerpt TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(import_id) REFERENCES imports(id) ON DELETE CASCADE,
  FOREIGN KEY(source_file_id) REFERENCES source_files(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_import_issues_import_id ON import_issues(import_id);
CREATE INDEX IF NOT EXISTS idx_import_issues_created_at ON import_issues(created_at DESC);

CREATE TABLE IF NOT EXISTS exports (
  id TEXT PRIMARY KEY,
  format TEXT NOT NULL,
  scope_json TEXT NOT NULL,
  dest_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);
