//! Statistics — per-tick aggregate KPI counters.

/// Aggregate statistics for one simulation tick.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Statistics {
    /// Current tick number.
    pub tick: u64,
    /// Number of active mobile terminals this tick.
    pub active_mobiles: u32,
    /// Cumulative arrivals since simulation start.
    pub total_arrivals: u64,
    /// Cumulative successful departures since simulation start.
    pub total_finished: u64,
    /// Cumulative mobiles dropped (poor SINR / no channel) since start.
    pub total_dropped: u64,
    /// Cumulative mobiles that could not be admitted since start.
    pub total_blocked: u64,
    /// Cumulative handoff attempts since simulation start.
    pub handoff_attempts: u64,
    /// Cumulative successful handoffs since simulation start.
    pub handoff_successes: u64,
    /// Cumulative failed handoffs since simulation start.
    pub handoff_failures: u64,
    /// Rolling mean SINR across all mobiles in dB.
    pub avg_sinr_db: f32,
    /// Rolling mean downlink throughput across all mobiles in Mbps.
    pub avg_throughput_mbps: f32,
    /// Rolling mean latency across all mobiles in ms.
    pub avg_latency_ms: f32,
    /// Cumulative SLA violations since simulation start.
    pub sla_violations: u64,
}

impl Statistics {
    /// Fraction of handoff attempts that succeeded (0.0 when no attempts).
    pub fn handoff_success_rate(&self) -> f32 {
        if self.handoff_attempts == 0 {
            0.0
        } else {
            self.handoff_successes as f32 / self.handoff_attempts as f32
        }
    }

    /// Fraction of arriving mobiles that were blocked (0.0 when no arrivals).
    pub fn block_rate(&self) -> f32 {
        if self.total_arrivals == 0 {
            0.0
        } else {
            self.total_blocked as f32 / self.total_arrivals as f32
        }
    }

    /// Fraction of arriving mobiles that were later dropped (0.0 when no arrivals).
    pub fn drop_rate(&self) -> f32 {
        if self.total_arrivals == 0 {
            0.0
        } else {
            self.total_dropped as f32 / self.total_arrivals as f32
        }
    }

    /// Serialise this statistics record as a CSV row.
    ///
    /// Format: `tick,active_mobiles,total_arrivals,total_dropped,block_rate,ho_success_rate,avg_sinr_db,avg_throughput_mbps,avg_latency_ms,sla_violations`
    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{}",
            self.tick,
            self.active_mobiles,
            self.total_arrivals,
            self.total_dropped,
            self.block_rate(),
            self.handoff_success_rate(),
            self.avg_sinr_db,
            self.avg_throughput_mbps,
            self.avg_latency_ms,
            self.sla_violations,
        )
    }
}

// ---------------------------------------------------------------------------
// CSV export helper
// ---------------------------------------------------------------------------

/// Export KPI data from a slice of replay frames to a CSV file.
pub fn export_kpi_csv(
    frames: &[crate::sim::replay::ReplayFrame],
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "tick,active_mobiles,total_arrivals,total_dropped,block_rate,ho_success_rate,avg_sinr_db,avg_throughput_mbps,avg_latency_ms,sla_violations")?;
    for frame in frames {
        writeln!(file, "{}", frame.stats.to_csv_row())?;
    }
    Ok(())
}
