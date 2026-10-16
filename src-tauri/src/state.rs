use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub running: Arc<Mutex<HashMap<String, u32>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
