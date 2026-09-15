//! Local power policy. Peer/infra advertisements never grant authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Saver,
    Beacon,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Auto,
    Normal,
    Saver,
    Beacon,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Android,
    Ios,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inputs {
    pub battery_percent: u8,
    pub charging: bool,
    /// Locally discovered peers, not a received ANNOUNCE peer-count claim.
    pub visible_peers: usize,
    pub auto_beacon: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameters {
    pub mode: Mode,
    pub max_links: usize,
    pub announce_ms: u64,
    pub scan_on_ms: u64,
    /// Zero means continuous scan; native lifecycle restrictions still apply.
    pub scan_off_ms: u64,
    pub infra: bool,
    pub beacon_battery_exit: bool,
}
pub struct Policy {
    platform: Platform,
    setting: Setting,
    mode: Mode,
    candidate: Option<(Mode, u64)>,
    now: u64,
}
impl Policy {
    pub fn new(platform: Platform, now: u64) -> Self {
        Self {
            platform,
            setting: Setting::Auto,
            mode: Mode::Normal,
            candidate: None,
            now,
        }
    }
    pub fn set(
        &mut self,
        setting: Setting,
        input: Inputs,
        now: u64,
    ) -> Result<Parameters, &'static str> {
        self.validate(input, now)?;
        if self.platform == Platform::Ios && setting == Setting::Beacon {
            return Err("Beacon requires Android");
        }
        self.setting = setting;
        self.candidate = None;
        if setting != Setting::Auto {
            self.mode = match setting {
                Setting::Saver => Mode::Saver,
                Setting::Beacon => Mode::Beacon,
                _ => Mode::Normal,
            };
        }
        self.update(input, now)
    }
    fn validate(&self, input: Inputs, now: u64) -> Result<(), &'static str> {
        if now < self.now || now > u64::MAX - 60_000 || input.battery_percent > 100 {
            return Err("invalid power input/time");
        }
        Ok(())
    }
    pub fn update(&mut self, input: Inputs, now: u64) -> Result<Parameters, &'static str> {
        self.validate(input, now)?;
        self.now = now;
        let battery_exit =
            self.mode == Mode::Beacon && !input.charging && input.battery_percent <= 30;
        if battery_exit {
            self.setting = Setting::Normal;
            self.mode = Mode::Normal;
            self.candidate = None;
        }
        let desired = match self.setting {
            Setting::Normal => Mode::Normal,
            Setting::Saver => Mode::Saver,
            Setting::Beacon => Mode::Beacon,
            Setting::Auto if input.charging => {
                if input.auto_beacon && self.platform == Platform::Android {
                    Mode::Beacon
                } else {
                    Mode::Normal
                }
            }
            Setting::Auto
                if input.battery_percent < 20
                    || (input.battery_percent <= 40 && input.visible_peers >= 4) =>
            {
                Mode::Saver
            }
            Setting::Auto => Mode::Normal,
        };
        if desired == self.mode {
            self.candidate = None;
        } else if let Some((mode, since)) = self.candidate.filter(|(m, _)| *m == desired) {
            if now - since >= 60_000 {
                self.mode = mode;
                self.candidate = None;
            }
        } else {
            self.candidate = Some((desired, now));
        }
        Ok(parameters(
            self.platform,
            self.mode,
            input.charging,
            battery_exit,
        ))
    }
}
pub fn parameters(
    platform: Platform,
    mode: Mode,
    charging: bool,
    beacon_battery_exit: bool,
) -> Parameters {
    Parameters {
        mode,
        max_links: match (platform, mode) {
            (_, Mode::Saver) => 3,
            (Platform::Android, Mode::Beacon) => 8,
            (Platform::Android, _) => 6,
            _ => 4,
        },
        announce_ms: if mode == Mode::Saver { 60_000 } else { 30_000 },
        scan_on_ms: if mode == Mode::Saver { 10_000 } else { 0 },
        scan_off_ms: if mode == Mode::Saver { 50_000 } else { 0 },
        infra: platform == Platform::Android && mode == Mode::Beacon && charging,
        beacon_battery_exit,
    }
}
