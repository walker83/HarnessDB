use dashmap::DashMap;

#[derive(Debug, Default)]
pub struct TransactionState {
    pub tran_count: u32,
    pub savepoints: Vec<String>,
    pub is_active: bool,
}

pub struct TsqlTransactionManager {
    sessions: DashMap<u32, TransactionState>,
}

impl TsqlTransactionManager {
    pub fn new() -> Self {
        Self { sessions: DashMap::new() }
    }

    pub fn begin_tran(&self, conn_id: u32) {
        let mut state = self.sessions.entry(conn_id).or_insert_with(TransactionState::default);
        state.tran_count += 1;
        state.is_active = true;
    }

    pub fn commit(&self, conn_id: u32) -> bool {
        let mut state = self.sessions.entry(conn_id).or_insert_with(TransactionState::default);
        if state.tran_count > 0 {
            state.tran_count -= 1;
        }
        if state.tran_count == 0 {
            state.is_active = false;
            state.savepoints.clear();
            true  // actually committed
        } else {
            false
        }
    }

    pub fn rollback(&self, conn_id: u32) {
        if let Some(mut state) = self.sessions.get_mut(&conn_id) {
            state.tran_count = 0;
            state.is_active = false;
            state.savepoints.clear();
        }
    }

    pub fn save_tran(&self, conn_id: u32, name: &str) {
        let mut state = self.sessions.entry(conn_id).or_insert_with(TransactionState::default);
        state.savepoints.push(name.to_string());
    }

    pub fn get_tran_count(&self, conn_id: u32) -> u32 {
        self.sessions.get(&conn_id).map(|s| s.tran_count).unwrap_or(0)
    }
}

impl Default for TsqlTransactionManager {
    fn default() -> Self { Self::new() }
}
