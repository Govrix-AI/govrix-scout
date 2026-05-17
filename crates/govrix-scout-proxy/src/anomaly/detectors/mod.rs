//! Detector implementations. Each module owns one detector and its tests.

pub mod behavioral;
pub mod cost;
pub mod error_rate;
pub mod exfil;
pub mod fanout;
pub mod latency;
pub mod legacy;
pub mod security;

pub use behavioral::BehavioralDetector;
pub use cost::CostDetector;
pub use error_rate::ErrorRateDetector;
pub use exfil::ExfilDetector;
pub use fanout::FanoutDetector;
pub use latency::LatencyDetector;
pub use legacy::LegacyDetector;
pub use security::SecurityDetector;
