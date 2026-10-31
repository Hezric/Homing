use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedNetwork {
    pub name: String,
    pub profile: String,
}
#[derive(Clone, Debug, Default)]
pub struct SavedNetworks {
    pub networks: Vec<SavedNetwork>,
    pub current: Option<SavedNetwork>,
}
#[derive(Serialize)]
pub struct NetworkChoices {
    pub names: Vec<String>,
    pub current: Option<String>,
}
impl SavedNetworks {
    pub fn choices(&self) -> NetworkChoices {
        let mut names = self
            .networks
            .iter()
            .map(|network| network.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        NetworkChoices {
            names,
            current: self
                .current
                .as_ref()
                .filter(|current| self.networks.contains(current))
                .map(|current| current.name.clone()),
        }
    }
    pub fn resolve(&self, name: &str) -> Result<SavedNetwork, String> {
        if let Some(current) = self
            .current
            .as_ref()
            .filter(|current| current.name == name && self.networks.contains(current))
        {
            return Ok(current.clone());
        }
        let mut matches = self
            .networks
            .iter()
            .filter(|network| network.name == name)
            .cloned()
            .collect::<Vec<_>>();
        matches.dedup();
        match matches.as_slice() {
            [network] => Ok(network.clone()),
            [] => Err("networkNotSaved".into()),
            _ => Err("networkAmbiguous".into()),
        }
    }
    pub fn current_for(&self, name: Option<&str>, profile: Option<&str>) -> Option<SavedNetwork> {
        let mut matches = self
            .networks
            .iter()
            .filter(|network| match profile {
                Some(profile) => network.profile == profile,
                None => name == Some(network.name.as_str()),
            })
            .cloned()
            .collect::<Vec<_>>();
        matches.dedup();
        if matches.len() > 1 {
            matches.retain(|network| name == Some(network.name.as_str()));
        }
        if matches.len() == 1 {
            matches.pop()
        } else {
            None
        }
    }
    pub fn name_for_profile(&self, profile: &str) -> Option<String> {
        let mut names = self
            .networks
            .iter()
            .filter(|network| network.profile == profile)
            .map(|network| network.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        if names.len() == 1 {
            names.pop()
        } else {
            None
        }
    }
}
