use crate::settings::Settings;
use serde::Serialize;
#[derive(Clone, Default)]
pub struct Network {
    pub current: Option<String>,
    pub interface_text: String,
}
pub trait WifiBackend {
    fn saved_networks(&mut self) -> Result<crate::networks::SavedNetworks, String> {
        Err("networksUnavailable".into())
    }
    fn notification_error(&self) -> Option<String> {
        None
    }
    fn inspect(&mut self) -> Result<Network, String>;
    fn connect(&mut self, profile: &str) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Connection {
    Connected,
    Reconnecting,
    Disconnected,
    Unreadable,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub current: Option<String>,
    #[serde(skip_serializing)]
    pub profile: String,
    pub configured: bool,
    pub target_name: Option<String>,
    pub connection: Connection,
    pub pause_remaining: u64,
    pub pause_minutes: u16,
    pub last_reconnect: Option<u64>,
    pub language_mode: String,
    pub error: Option<String>,
}
pub struct Reconnector {
    pub settings: Settings,
    current: Option<String>,
    connection: Connection,
    pause_until: Option<u64>,
    retry_at: Option<u64>,
    observation_at: Option<u64>,
    last_reconnect: Option<u64>,
    error: Option<String>,
}
impl Reconnector {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            current: None,
            connection: Connection::Disconnected,
            pause_until: None,
            retry_at: None,
            observation_at: None,
            last_reconnect: None,
            error: None,
        }
    }
    pub fn reconcile(&mut self, wifi: &mut impl WifiBackend, now: impl Fn() -> u64, unix: u64) {
        let network = match wifi.inspect() {
            Ok(network) => network,
            Err(error) => {
                self.current = None;
                self.connection = Connection::Unreadable;
                self.error = Some(format!("Can't read Wi-Fi state: {error}"));
                self.observation_at = Some(now() + 1000);
                return;
            }
        };
        self.observation_at = None;
        let observation_error = wifi.notification_error();
        let notifications_missing = wifi.notification_error().is_some();
        if let Some(error) = &observation_error {
            self.error = Some(error.clone());
        }
        let at = now();
        self.current = network.current;
        if self.settings.profile.is_empty() {
            self.connection = Connection::Disconnected;
            self.retry_at = notifications_missing.then(|| now() + 61_000);
            self.error = observation_error;
            return;
        }
        let target = if self.settings.network.is_empty() {
            &self.settings.profile
        } else {
            &self.settings.network
        };
        if network.interface_text.contains(target) {
            self.connection = Connection::Connected;
            self.retry_at = notifications_missing.then(|| now() + 1000);
            self.error = observation_error;
            return;
        }
        if self.pause_until.is_some_and(|deadline| at < deadline) {
            self.connection = Connection::Disconnected;
            self.retry_at = notifications_missing.then(|| now() + 1000);
            self.error = observation_error;
            return;
        }
        if self.pause_until.take().is_some() {
            self.retry_at = None;
        }
        if self.retry_at.is_some_and(|deadline| at < deadline) {
            if self.connection == Connection::Unreadable {
                self.connection = Connection::Disconnected;
                self.error = observation_error;
            }
            return;
        }
        match wifi.connect(&self.settings.profile) {
            Ok(()) => {
                self.connection = Connection::Reconnecting;
                self.last_reconnect = Some(unix);
                self.error = observation_error;
                self.retry_at = Some(now() + 1000);
            }
            Err(error) => {
                self.connection = Connection::Disconnected;
                self.error = Some(match observation_error {
                    Some(query) => format!("{query}\n{error}"),
                    None => error,
                });
                self.retry_at = Some(now() + 61_000);
            }
        }
    }
    pub fn state(&self, now: u64) -> State {
        State {
            current: self.current.clone(),
            profile: self.settings.profile.clone(),
            configured: !self.settings.profile.is_empty(),
            target_name: (!self.settings.network.is_empty()).then(|| self.settings.network.clone()),
            connection: self.connection,
            pause_remaining: self
                .pause_until
                .unwrap_or(0)
                .saturating_sub(now)
                .div_ceil(1000),
            pause_minutes: self.settings.pause_minutes,
            last_reconnect: self.last_reconnect,
            language_mode: self.settings.language.clone(),
            error: self.error.clone(),
        }
    }
    pub fn reset_retry(&mut self) {
        self.retry_at = None;
    }
    pub fn pause(&mut self, now: u64) {
        self.pause_until = Some(now + self.settings.pause_minutes as u64 * 60_000);
    }
    pub fn resume(&mut self) {
        self.pause_until = None;
        self.retry_at = None;
    }
    pub fn deadline(&self, now: u64) -> Option<u64> {
        self.pause_until
            .filter(|end| *end > now)
            .into_iter()
            .chain(
                self.retry_at
                    .filter(|_| self.connection != Connection::Unreadable),
            )
            .chain(self.observation_at)
            .min()
    }
}
