use std::{fs, io::Write, path::Path};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub profile: String,
    pub language: String,
    pub pause_minutes: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: String::new(),
            language: "system".into(),
            pause_minutes: 60,
        }
    }
}
pub fn validate_profile(profile: &str) -> Result<(), String> {
    if profile.trim().is_empty()
        || profile.contains(['\0', '\r', '\n'])
        || profile.encode_utf16().count() > 255
    {
        Err("Enter a saved Wi-Fi profile name (1–255 characters).".into())
    } else {
        Ok(())
    }
}
impl Settings {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut settings = Self::default();
        let mut section = "";
        for line in text.trim_start_matches('\u{feff}').lines().map(str::trim) {
            if line.is_empty() || line.starts_with([';', '#']) {
                continue;
            }
            if let Some(name) = line.strip_prefix('[') {
                section = name.strip_suffix(']').ok_or("Malformed INI section")?;
                continue;
            }
            let (key, value) = line.split_once('=').ok_or("Malformed INI setting")?;
            if section != "Settings" {
                continue;
            }
            match key.trim() {
                "ProfileHex" => {
                    let value = value.trim();
                    if !value.is_ascii() || !value.len().is_multiple_of(2) {
                        return Err("Invalid profile encoding".into());
                    }
                    let bytes = (0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| "Invalid profile encoding")?;
                    settings.profile =
                        String::from_utf8(bytes).map_err(|_| "Invalid profile encoding")?;
                    if !settings.profile.is_empty() {
                        validate_profile(&settings.profile)?;
                    }
                }
                "Language" => settings.language = value.trim().into(),
                "PauseMinutes" => {
                    settings.pause_minutes =
                        value.trim().parse().map_err(|_| "Invalid pause duration")?
                }
                _ => {}
            }
        }
        if !["system", "en", "zh"].contains(&settings.language.as_str()) {
            return Err("Invalid language".into());
        }
        if !(1..=720).contains(&settings.pause_minutes) {
            return Err("Pause duration must be 1 to 720 whole minutes".into());
        }
        Ok(settings)
    }
    pub fn encode(&self) -> String {
        let hex: String = self
            .profile
            .as_bytes()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        format!(
            "[Settings]\nProfileHex={hex}\nLanguage={}\nPauseMinutes={}\n",
            self.language, self.pause_minutes
        )
    }
    pub fn load(path: &Path) -> (Self, Option<String>) {
        match fs::read_to_string(path) {
            Ok(text) => match Self::parse(&text) {
                Ok(settings) => (settings, None),
                Err(error) => (
                    Self::default(),
                    Some(format!(
                        "Settings could not be read; reconnect is off. {error}"
                    )),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(error) => (
                Self::default(),
                Some(format!(
                    "Settings could not be read; reconnect is off. {error}"
                )),
            ),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        Self::parse(&self.encode())?;
        let parent = path.parent().ok_or("Settings directory is missing")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("ini.tmp");
        let result = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(self.encode().as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|e| format!("Settings could not be saved: {e}"))
    }
}
