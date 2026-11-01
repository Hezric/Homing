use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
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
        let settings: Self = toml::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("Invalid TOML settings: {e}"))?;
        if !settings.profile.is_empty() {
            validate_profile(&settings.profile)?;
        }
        if !settings.network.is_empty() {
            validate_network(&settings.network)?;
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
        toml::to_string_pretty(self).expect("settings contain only TOML-compatible types")
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
        let temporary = path.with_extension("toml.tmp");
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
