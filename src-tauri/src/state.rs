use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Clone)]
pub struct AppState {
    db_path: PathBuf,
    import_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            import_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    pub fn try_start_import(&self) -> bool {
        !self.import_running.swap(true, Ordering::SeqCst)
    }

    pub fn finish_import(&self) {
        self.import_running.store(false, Ordering::SeqCst);
    }
}
