use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RamMetrics {
    total: f64,
    available: f64,
    cache: f64,
    #[serde(rename = "swapTotal")]
    swap_total: f64,
    #[serde(rename = "swapUsed")]
    swap_used: f64,
}

#[allow(dead_code)]
impl RamMetrics {
    pub fn total(&self) -> f64 {
        self.total
    }
    pub fn available(&self) -> f64 {
        self.available
    }
    pub fn cache(&self) -> f64 {
        self.cache
    }
    pub fn swap_total(&self) -> f64 {
        self.swap_total
    }
    pub fn swap_used(&self) -> f64 {
        self.swap_used
    }
}
