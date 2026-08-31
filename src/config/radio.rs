//! RadioConfig — per-generation radio technology and traffic profile parameters.

use serde::{Deserialize, Serialize};

/// Supported radio access technologies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RadioTechnology {
    Gen3Umts,
    Gen4Lte,
    Gen5NrSub6,
    Gen5NrMmWave,
}

/// Mobile traffic application profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrafficProfile {
    VideoStream,
    CloudGaming,
    VoiceCall,
    Idle,
    WebBrowse,
}

impl TrafficProfile {
    /// Downlink bandwidth demand in Mbps.
    pub fn dl_demand_mbps(&self) -> f32 {
        match self {
            Self::VideoStream  => 15.0,
            Self::CloudGaming  => 50.0,
            Self::VoiceCall    => 0.1,
            Self::Idle         => 0.001,
            Self::WebBrowse    => 5.0,
        }
    }

    /// Uplink bandwidth demand in Mbps.
    pub fn ul_demand_mbps(&self) -> f32 {
        match self {
            Self::VideoStream  => 2.0,
            Self::CloudGaming  => 5.0,
            Self::VoiceCall    => 0.1,
            Self::Idle         => 0.001,
            Self::WebBrowse    => 1.0,
        }
    }

    /// Maximum tolerable one-way latency in milliseconds.
    pub fn latency_budget_ms(&self) -> f32 {
        match self {
            Self::VideoStream  => 100.0,
            Self::CloudGaming  => 20.0,
            Self::VoiceCall    => 150.0,
            Self::Idle         => 10000.0,
            Self::WebBrowse    => 200.0,
        }
    }

    /// Short human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::VideoStream  => "Video Stream",
            Self::CloudGaming  => "Cloud Gaming",
            Self::VoiceCall    => "Voice Call",
            Self::Idle         => "Idle",
            Self::WebBrowse    => "Web Browse",
        }
    }

    /// RGB colour used for rendering.
    pub fn color(&self) -> [u8; 3] {
        match self {
            Self::VideoStream  => [0, 120, 200],
            Self::CloudGaming  => [220, 50, 50],
            Self::VoiceCall    => [0, 180, 100],
            Self::Idle         => [150, 150, 150],
            Self::WebBrowse    => [200, 140, 0],
        }
    }
}

/// Radio channel configuration for a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioConfig {
    /// Selected radio access technology.
    pub technology: RadioTechnology,
    /// Number of base stations.
    pub no_base: u32,
    /// Number of channels per base station.
    pub no_channel: u32,
    /// Base station antenna height in metres.
    pub base_height_m: f32,
    /// Channel bandwidth in MHz.
    pub bandwidth_mhz: f32,
    /// Carrier frequency in GHz.
    pub frequency_ghz: f32,
    /// Number of MIMO spatial layers.
    pub mimo_layers: u8,
    /// Maximum transmit power in dBm (macro cell default).
    pub max_tx_power_dbm: f32,
    /// Receiver noise figure in dB.
    pub noise_figure_db: f32,
    /// Thermal noise power in dBm at the configured bandwidth.
    pub thermal_noise_dbm: f32,
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            technology: RadioTechnology::Gen4Lte,
            no_base: 12,
            no_channel: 30,
            base_height_m: 10.0,
            bandwidth_mhz: 20.0,
            frequency_ghz: 2.6,
            mimo_layers: 2,
            max_tx_power_dbm: 43.0,
            noise_figure_db: 7.0,
            thermal_noise_dbm: -107.0,
        }
    }
}

impl RadioConfig {
    /// Spectral efficiency factor for the selected technology.
    ///
    /// Gen3=0.4, Gen4=0.6, Gen5NrSub6=0.8, Gen5NrMmWave=0.9
    pub fn spectral_efficiency(&self) -> f32 {
        match self.technology {
            RadioTechnology::Gen3Umts       => 0.4,
            RadioTechnology::Gen4Lte        => 0.6,
            RadioTechnology::Gen5NrSub6     => 0.8,
            RadioTechnology::Gen5NrMmWave   => 0.9,
        }
    }

    /// Path loss exponent for the selected technology.
    ///
    /// Gen3=3.5, Gen4=3.5, Gen5NrSub6=3.5, Gen5NrMmWave=3.8
    pub fn path_loss_exponent(&self) -> f32 {
        match self.technology {
            RadioTechnology::Gen3Umts       => 3.5,
            RadioTechnology::Gen4Lte        => 3.5,
            RadioTechnology::Gen5NrSub6     => 3.5,
            RadioTechnology::Gen5NrMmWave   => 3.8,
        }
    }

    /// Typical round-trip time base in milliseconds.
    ///
    /// Gen3=150, Gen4=40, Gen5NrSub6=5, Gen5NrMmWave=2
    pub fn base_rtt_ms(&self) -> f32 {
        match self.technology {
            RadioTechnology::Gen3Umts       => 150.0,
            RadioTechnology::Gen4Lte        => 40.0,
            RadioTechnology::Gen5NrSub6     => 5.0,
            RadioTechnology::Gen5NrMmWave   => 2.0,
        }
    }
}
