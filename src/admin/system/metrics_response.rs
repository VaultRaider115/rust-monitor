use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    cpu_metrics::CpuMetrics, disk_metrics::DiskMetrics, network_metrics::NetworkMetrics,
    ram_metrics::RamMetrics,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct MetricsResponse {
    #[serde(rename = "CPU")]
    cpu: CpuMetrics,
    ram: RamMetrics,
    #[serde(rename = "disk")]
    disk: DiskMetrics,
    red: BTreeMap<String, NetworkMetrics>,
}

#[allow(dead_code)]
impl MetricsResponse {
    pub fn cpu(&self) -> &CpuMetrics {
        &self.cpu
    }
    pub fn ram(&self) -> &RamMetrics {
        &self.ram
    }
    pub fn disk(&self) -> &DiskMetrics {
        &self.disk
    }
    pub fn red(&self) -> &BTreeMap<String, NetworkMetrics> {
        &self.red
    }
}

#[cfg(test)]
mod tests {
    use super::MetricsResponse;

    #[test]
    fn deserializes_integer_and_decimal_metric_values() {
        let json = r#"{"CPU":{"usage":13.81993844911962},"disk":{"device":"nvme0n1","free":41145077760.0,"usage":111288516608.0,"usageP":63.02846145629883,"used":70143442944.0},"ram":{"available":6.733875274658203,"cache":4.729915618896484,"swapTotal":14.885738372802734,"swapUsed":3.94189453125,"total":14.885856628417969},"red":{"enp2s0":{"received":0.0,"receivedPackets":0,"transmitted":0.0,"trasmittedPackets":0}}}"#;

        let metrics: MetricsResponse = serde_json::from_str(json).unwrap();

        assert_eq!(metrics.cpu().usage(), 13.81993844911962);
        assert_eq!(metrics.ram().total(), 14.885856628417969);
        assert_eq!(metrics.disk().free(), 41145077760.0);
        assert_eq!(metrics.disk().usage_p(), 63.02846145629883);
        assert_eq!(metrics.red()["enp2s0"].received_packets(), 0.0);
        assert_eq!(metrics.red()["enp2s0"].transmitted_packets(), 0.0);
    }
}
