use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CpuMetrics {
    usage: f64,
}

#[allow(dead_code)]
impl CpuMetrics {
    pub fn usage(&self) -> f64 {
        self.usage
    }
}
