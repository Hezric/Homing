use crate::{
    core::{Network, WifiBackend},
    runtime::Message,
    settings::validate_profile,
};
use std::{ffi::c_void, os::windows::process::CommandExt, process::Command, sync::mpsc::Sender};
use windows::Win32::{
    Foundation::HANDLE,
    Globalization::{MultiByteToWideChar, CP_OEMCP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS},
    NetworkManagement::{
        IpHelper::{FreeMibTable, GetIfTable2, IF_TYPE_IEEE80211},
        Ndis::{IfOperStatusDown, IfOperStatusLowerLayerDown, IfOperStatusUp},
        WiFi::*,
    },
    Networking::NetworkListManager::{INetworkListManager, NetworkListManager},
    System::Com::{CoCreateInstance, CLSCTX_ALL},
};

pub struct WindowsWifi {
    handle: HANDLE,
    context: Box<Sender<Message>>,
    subscribed: bool,
    notification_error: Option<String>,
}
unsafe extern "system" fn changed(_: *mut L2_NOTIFICATION_DATA, context: *mut c_void) {
    // Windows owns the callback lifetime until unregister/close completes.
    if let Some(sender) = unsafe { context.cast::<Sender<Message>>().as_ref() } {
        let _ = sender.send(Message::NetworkChanged);
    }
}
impl WindowsWifi {
    pub fn new(sender: Sender<Message>) -> Self {
        let mut wifi = Self {
            handle: HANDLE::default(),
            context: Box::new(sender),
            subscribed: false,
            notification_error: None,
        };
        wifi.subscribe();
        wifi
    }
    fn subscribe(&mut self) {
        if self.subscribed {
            return;
        }
        if self.handle.is_invalid() {
            let mut version = 0;
            let code = unsafe { WlanOpenHandle(2, None, &mut version, &mut self.handle) };
            if code != 0 {
                self.notification_error = Some(format!("WLAN notifications are unavailable ({code}); using timed checks while retrying registration."));
                return;
            }
        }
        let code = unsafe {
            WlanRegisterNotification(
                self.handle,
                WLAN_NOTIFICATION_SOURCE_ACM,
                true,
                Some(changed),
                Some((&*self.context as *const Sender<Message>).cast()),
                None,
                None,
            )
        };
        if code != 0 {
            unsafe {
                WlanCloseHandle(self.handle, None);
            }
            self.handle = HANDLE::default();
        }
        self.subscribed = code == 0;
        self.notification_error = if self.subscribed {
            None
        } else {
            Some(format!("WLAN notification registration failed ({code}); using timed checks while retrying registration."))
        };
    }
}
impl Drop for WindowsWifi {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                WlanRegisterNotification(
                    self.handle,
                    WLAN_NOTIFICATION_SOURCE_NONE,
                    true,
                    None,
                    None,
                    None,
                    None,
                );
                WlanCloseHandle(self.handle, None);
            }
        }
    }
}
fn decode(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.trim_start_matches('\u{feff}').into();
    }
    let len =
        unsafe { MultiByteToWideChar(CP_OEMCP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None) };
    let mut wide = vec![0; len.max(0) as usize];
    unsafe {
        MultiByteToWideChar(
            CP_OEMCP,
            MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
            bytes,
            Some(&mut wide),
        );
    }
    String::from_utf16_lossy(&wide)
}
fn netsh(args: &[&str]) -> Result<String, String> {
    let executable = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("SystemRoot is not available")?
        .join("System32/netsh.exe");
    let output = Command::new(executable)
        .args(args)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Windows Wi-Fi command failed: {e}"))?;
    let text = decode(&output.stdout);
    if output.status.success() {
        Ok(text)
    } else {
        Err(format!(
            "Windows Wi-Fi command failed ({}). {}",
            output.status,
            text.trim()
        ))
    }
}
impl WifiBackend for WindowsWifi {
    fn notification_error(&self) -> Option<String> {
        self.notification_error.clone()
    }
    fn inspect(&mut self) -> Result<Network, String> {
        self.subscribe();
        inspect_network().map_err(|e| e.to_string())
    }
    fn connect(&mut self, profile: &str) -> Result<(), String> {
        validate_profile(profile)?;
        // Keep the old saved-profile selection without passing user text to cmd.exe.
        netsh(&["wlan", "connect", &format!("name={profile}")]).map(|_| ())
    }
}

// The worker owns the COM apartment; these observations do not require Location.
fn inspect_network() -> windows_core::Result<Network> {
    unsafe {
        let mut table = std::ptr::null_mut();
        GetIfTable2(&mut table).ok()?;
        let adapters: Vec<_> =
            std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
                .iter()
                // HardwareInterface is bit 0; filter-layer rows duplicate the radio.
                .filter(|row| {
                    row.Type == IF_TYPE_IEEE80211
                        && row.InterfaceAndOperStatusFlags._bitfield & 1 != 0
                })
                .copied()
                .collect();
        FreeMibTable(table.cast());
        if adapters.is_empty() {
            return Err(unreadable());
        }
        let mut names = Vec::new();
        for adapter in adapters {
            if adapter.OperStatus != IfOperStatusUp {
                adapter_name(adapter.OperStatus, None)?;
                continue;
            }
            let manager: INetworkListManager =
                CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)?;
            let connections = manager.GetNetworkConnections()?;
            let mut name = None;
            loop {
                let mut item = [None];
                let mut fetched = 0;
                connections.Next(&mut item, Some(&mut fetched))?;
                if fetched == 0 {
                    break;
                }
                let connection = item[0].take().ok_or_else(unreadable)?;
                if connection.GetAdapterId()? == adapter.InterfaceGuid
                    && connection.IsConnected()?.as_bool()
                {
                    // GetName is the editable profile label (Windows can append " 2", " 3", ...).
                    let value = connection.GetNetwork()?.GetDescription()?.to_string();
                    if !value.is_empty() {
                        name = Some(value);
                        break;
                    }
                }
            }
            names.extend(adapter_name(adapter.OperStatus, name)?);
        }
        Ok(Network {
            current: names.first().cloned(),
            interface_text: names.join("\n"),
        })
    }
}
fn unreadable() -> windows_core::Error {
    windows_core::Error::new(
        windows_core::HRESULT(0x80004005u32 as i32),
        "Wi-Fi adapter or network name is unavailable",
    )
}
fn adapter_name(
    status: windows::Win32::NetworkManagement::Ndis::IF_OPER_STATUS,
    name: Option<String>,
) -> windows_core::Result<Option<String>> {
    if [IfOperStatusDown, IfOperStatusLowerLayerDown].contains(&status) {
        Ok(None)
    } else if status == IfOperStatusUp {
        name.filter(|name| !name.is_empty())
            .map(Some)
            .ok_or_else(unreadable)
    } else {
        Err(unreadable())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_known_adapter_states_allow_reconnect_decisions() {
        use windows::Win32::NetworkManagement::Ndis::{
            IfOperStatusNotPresent, IfOperStatusUnknown,
        };
        for status in [IfOperStatusDown, IfOperStatusLowerLayerDown] {
            assert_eq!(adapter_name(status, None).unwrap(), None);
        }
        for status in [IfOperStatusNotPresent, IfOperStatusUnknown, IfOperStatusUp] {
            assert!(adapter_name(status, None).is_err());
        }
        assert!(adapter_name(IfOperStatusUp, Some(String::new())).is_err());
        // Network descriptions keep actual numeric endings; never strip a suffix.
        for name in ["Home", "HomeNetwork_5", "Studio 3"] {
            assert_eq!(
                adapter_name(IfOperStatusUp, Some(name.into()))
                    .unwrap()
                    .as_deref(),
                Some(name)
            );
        }
    }
    #[test]
    #[ignore = "Read-only Windows WLAN diagnostic; run explicitly on a Wi-Fi machine"]
    fn read_only_subscription_and_query() {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
            .ok()
            .unwrap();
        }
        let (sender, _receive) = std::sync::mpsc::channel();
        let mut wifi = WindowsWifi::new(sender);
        assert!(wifi.notification_error().is_none());
        let network = wifi.inspect().unwrap();
        assert!(!network.interface_text.is_empty());
        println!(
            "Read-only adapter/profile query passed; current network: {:?}",
            network.current
        );
        drop(wifi);
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
