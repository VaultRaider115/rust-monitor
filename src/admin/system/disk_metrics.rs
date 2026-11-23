use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DiskMetrics {
    device: String,
    free: f64,
    used: f64,
    usage: f64,
    #[serde(rename = "usageP")]
    usage_p: f64,
}

#[allow(dead_code)]
impl DiskMetrics {
    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn free(&self) -> f64 {
        self.free
    }
    pub fn used(&self) -> f64 {
        self.used
    }
    pub fn usage(&self) -> f64 {
        self.usage
    }
    pub fn usage_p(&self) -> f64 {
        self.usage_p
    }
}
