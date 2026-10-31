use std::{fs, io::Write, path::Path};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub profile: String,
    pub network: String,
    pub language: String,
    pub pause_minutes: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: String::new(),
            network: String::new(),
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
        Err("The saved network connection information is invalid.".into())
    } else {
        Ok(())
    }
}
pub fn validate_network(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.contains(['\0', '\r', '\n']) || name.len() > 32 {
        Err("networkInvalid".into())
    } else {
        Ok(())
    }
}
impl Settings {
    pub fn set_target(
        &mut self,
        network: Option<&str>,
        wifi: &mut impl crate::core::WifiBackend,
        path: &Path,
    ) -> Result<(), String> {
        if let Some(network) = network {
            validate_network(network)?;
        }
        let saved = wifi.saved_networks().map_err(|_| "networksUnavailable")?;
        let selected = match network {
            Some(network) => saved.resolve(network)?,
            None => saved
                .current
                .as_ref()
                .filter(|current| saved.networks.contains(current))
                .cloned()
                .ok_or("currentNetworkUnavailable")?,
        };
        validate_network(&selected.name)?;
        validate_profile(&selected.profile).map_err(|_| "networkNotSaved")?;
        let mut next = self.clone();
        next.profile = selected.profile;
        next.network = selected.name;
        next.save(path)?;
        *self = next;
        Ok(())
    }
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
                "ProfileHex" | "NetworkHex" => {
                    let value = value.trim();
                    if !value.is_ascii() || !value.len().is_multiple_of(2) {
                        return Err("Invalid profile encoding".into());
                    }
                    let bytes = (0..value.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&value[i..i + 2], 16))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| "Invalid profile encoding")?;
                    let value = String::from_utf8(bytes).map_err(|_| "Invalid profile encoding")?;
                    if key.trim() == "ProfileHex" {
                        if !value.is_empty() {
                            validate_profile(&value)?;
                        }
                        settings.profile = value;
                    } else {
                        if !value.is_empty() {
                            validate_network(&value)?;
                        }
                        settings.network = value;
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
        let network_hex: String = self
            .network
            .as_bytes()
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect();
        format!(
            "[Settings]\nProfileHex={hex}\nNetworkHex={network_hex}\nLanguage={}\nPauseMinutes={}\n",
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
