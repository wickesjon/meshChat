//! Local, session-only counters. No identity/content dimensions or delivery claims.
use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct TransportStats {
    pub elapsed_ms: u64,
    pub received_frames: u64,
    pub received_bytes: u64,
    pub received_packets: u64,
    pub received_chat_packets: u64,
    pub scheduled_frames: u64,
    pub completed_frames: u64,
    pub completed_bytes: u64,
    pub completed_objects: u64,
    pub relayed_chat_copies: u64,
    pub battery_percent: Option<u8>,
    pub charging: bool,
    pub power_mode: String,
}
pub(super) struct Stats {
    pub values: TransportStats,
    since: u64,
    power_at: Option<u64>,
}
impl Stats {
    pub fn new(now: u64) -> Self {
        Self {
            values: TransportStats {
                power_mode: "Normal".into(),
                ..Default::default()
            },
            since: now,
            power_at: None,
        }
    }
    pub fn power(&mut self, params: power::Parameters, battery: u8, charging: bool, now: u64) {
        self.values.battery_percent = Some(battery);
        self.values.charging = charging;
        self.values.power_mode = match params.mode {
            power::Mode::Normal => "Normal",
            power::Mode::Saver => "Saver",
            power::Mode::Beacon => "Beacon",
        }
        .into();
        self.power_at = Some(now);
    }
    fn snapshot(&self, now: u64) -> TransportStats {
        let mut result = self.values.clone();
        result.elapsed_ms = now - self.since;
        if self.power_at.is_none_or(|at| now - at > 60_000) {
            result.battery_percent = None;
            result.charging = false;
        }
        result
    }
}
#[uniffi::export]
impl NativeTransport {
    pub fn contribution_stats(&self, now: u64) -> Result<TransportStats, TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        Ok(s.stats.snapshot(now))
    }
    /// Only presentation counters reset: no resource credits, queues or trust.
    pub fn reset_contribution_stats(&self, now: u64) -> Result<(), TransportError> {
        let mut s = self.state.lock().map_err(|_| TransportError::Unavailable)?;
        s.clock(now)?;
        let old = s.stats.snapshot(now);
        s.stats.values = TransportStats {
            battery_percent: old.battery_percent,
            charging: old.charging,
            power_mode: old.power_mode,
            ..Default::default()
        };
        s.stats.since = now;
        Ok(())
    }
}

/// Fixed allowlist for the preview/export: no free-form or private input.
#[uniffi::export]
pub fn contribution_share_text(
    stats: TransportStats,
    received: bool,
    sent: bool,
    relayed: bool,
) -> String {
    let mut lines = vec![
        "MeshChat · local contribution".to_string(),
        format!("Session window: {} minutes", stats.elapsed_ms / 60_000),
    ];
    if received {
        lines.push(format!("Received frames: {}", stats.received_frames));
        lines.push(format!("Reassembled packets: {}", stats.received_packets));
        lines.push("Repeated copies included; not unique messages.".into());
    }
    if sent {
        lines.push(format!(
            "Native frame completions: {}",
            stats.completed_frames
        ));
        lines.push(format!(
            "Completed outgoing objects: {}",
            stats.completed_objects
        ));
    }
    if relayed {
        lines.push(format!(
            "Live chat copies relayed: {}",
            stats.relayed_chat_copies
        ));
    }
    lines.push("Local radio activity. Delivery unknown.".into());
    lines.push("No people, reach or bridge estimate.".into());
    lines.join("\n")
}
