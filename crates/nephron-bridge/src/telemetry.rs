//! Lightweight Linux `/proc/meminfo` telemetry for 0-swap SBC RAM monitoring.

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTelemetry {
    pub total_mib: u64,
    pub available_mib: u64,
    pub used_mib: u64,
    pub cpu_temp_c: Option<f32>,
}

impl SystemTelemetry {
    pub fn sample() -> Self {
        #[cfg(target_os = "linux")]
        if let Ok(raw) = std::fs::read_to_string("/proc/meminfo") {
            let mut t = parse_meminfo(&raw);
            t.cpu_temp_c = read_cpu_temp();
            return t;
        }
        Self::default()
    }

    /// Returns true if available RAM covers `model_gib` plus `headroom_mib` (0-swap SBC guard).
    pub fn can_fit_gib(self, model_gib: f32, headroom_mib: u64) -> bool {
        if self.total_mib == 0 {
            return true;
        }
        let needed_mib = (model_gib * 1024.0) as u64 + headroom_mib;
        self.available_mib >= needed_mib
    }
}

pub fn parse_meminfo(raw: &str) -> SystemTelemetry {
    let mut total_kib = 0u64;
    let mut avail_kib = 0u64;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kib = parse_kib(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail_kib = parse_kib(rest);
        }
    }
    let total_mib = total_kib / 1024;
    let available_mib = avail_kib / 1024;
    SystemTelemetry {
        total_mib,
        available_mib,
        used_mib: total_mib.saturating_sub(available_mib),
        cpu_temp_c: None,
    }
}

#[cfg(target_os = "linux")]
fn read_cpu_temp() -> Option<f32> {
    let raw = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").ok()?;
    let milli: f32 = raw.trim().parse().ok()?;
    Some(milli / 1000.0)
}

fn parse_kib(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_meminfo() {
        let sample = "MemTotal:       14680064 kB\nMemFree:          180000 kB\nMemAvailable:    6553600 kB\n";
        let t = parse_meminfo(sample);
        assert_eq!(t.total_mib, 14336);
        assert_eq!(t.available_mib, 6400);
        assert_eq!(t.used_mib, 7936);
        assert!(t.can_fit_gib(2.1, 1024));
        assert!(!t.can_fit_gib(6.2, 1024));
    }
}
