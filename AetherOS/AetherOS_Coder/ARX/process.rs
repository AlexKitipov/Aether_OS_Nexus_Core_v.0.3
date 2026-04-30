pub struct AppProcess {
    pub id: u64,
}

pub fn spawn_process(id: u64) -> AppProcess {
    AppProcess { id }
}
