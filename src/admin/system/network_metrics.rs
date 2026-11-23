use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkMetrics {
    #[serde(alias = "recived")]
    received: f64,
    #[serde(
        default,
        rename = "receivedPackets",
        alias = "received_packets",
        alias = "recivedPackets"
    )]
    received_packets: f64,
    #[serde(alias = "trasmitted")]
    transmitted: f64,
    #[serde(
        default,
        rename = "transmittedPackets",
        alias = "transmitted_packets",
        alias = "trasmitted_packets",
        alias = "trasmittedPackets"
    )]
    transmitted_packets: f64,
}

#[allow(dead_code)]
impl NetworkMetrics {
    pub fn received(&self) -> f64 {
        self.received
    }
    pub fn received_packets(&self) -> f64 {
        self.received_packets
    }
    pub fn transmitted(&self) -> f64 {
        self.transmitted
    }
    pub fn transmitted_packets(&self) -> f64 {
        self.transmitted_packets
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkMetrics;

    #[test]
    fn defaults_missing_packet_counters() {
        let metrics: NetworkMetrics =
            serde_json::from_str(r#"{"received":12.5,"transmitted":3.25}"#).unwrap();

        assert_eq!(metrics.received(), 12.5);
        assert_eq!(metrics.received_packets(), 0.0);
        assert_eq!(metrics.transmitted(), 3.25);
        assert_eq!(metrics.transmitted_packets(), 0.0);
    }

    #[test]
    fn accepts_typoed_server_field_names() {
        let metrics: NetworkMetrics = serde_json::from_str(
            r#"{"recived":1.5,"recivedPackets":42.0,"transmitted":7.5,"trasmittedPackets":9.0}"#,
        )
        .unwrap();

        assert_eq!(metrics.received(), 1.5);
        assert_eq!(metrics.received_packets(), 42.0);
        assert_eq!(metrics.transmitted(), 7.5);
        assert_eq!(metrics.transmitted_packets(), 9.0);
    }
}
