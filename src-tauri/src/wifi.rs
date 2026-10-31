use crate::{
    core::{Network, WifiBackend},
    networks::{SavedNetwork, SavedNetworks},
    runtime::Message,
    settings::validate_profile,
};
use std::{ffi::c_void, os::windows::process::CommandExt, process::Command, sync::mpsc::Sender};
use windows::Win32::{
    Foundation::HANDLE,
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
fn netsh(args: &[&str]) -> Result<(), String> {
    let executable = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .ok_or("SystemRoot is not available")?
        .join("System32/netsh.exe");
    let output = Command::new(executable)
        .args(args)
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Windows Wi-Fi command failed: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Windows Wi-Fi command failed ({}). Check the saved network in Windows.",
            output.status
        ))
    }
}
impl WifiBackend for WindowsWifi {
    fn saved_networks(&mut self) -> Result<SavedNetworks, String> {
        self.subscribe();
        if self.handle.is_invalid() {
            return Err("networksUnavailable".into());
        }
        read_saved_networks(self.handle)
    }
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
        netsh(&["wlan", "connect", &format!("name={profile}")])
    }
}

// WLAN owns these allocations, including on early-return error paths.
struct WlanMemory<T>(*mut T);
impl<T> Drop for WlanMemory<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WlanFreeMemory(self.0.cast()) }
        }
    }
}
fn profile_name(value: &[u16]) -> Result<String, String> {
    let end = value
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(value.len());
    String::from_utf16(&value[..end]).map_err(|_| "networksUnavailable".into())
}
fn current_access_allowed() -> bool {
    use windows::Security::Authorization::AppCapabilityAccess::{
        AppCapability, AppCapabilityAccessStatus,
    };
    // CheckAccess never prompts; a current-connection query can prompt without this gate.
    AppCapability::Create(&"wiFiControl".into())
        .and_then(|capability| capability.CheckAccess())
        .is_ok_and(|status| status == AppCapabilityAccessStatus::Allowed)
}
fn current_profile(
    handle: HANDLE,
    adapter: windows_core::GUID,
    allowed: bool,
) -> Option<(windows_core::GUID, String)> {
    if !allowed || handle.is_invalid() {
        return None;
    }
    unsafe {
        let mut data = WlanMemory(std::ptr::null_mut::<std::ffi::c_void>());
        let mut size = 0;
        let code = WlanQueryInterface(
            handle,
            &adapter,
            wlan_intf_opcode_current_connection,
            None,
            &mut size,
            &mut data.0,
            None,
        );
        if code != 0
            || data.0.is_null()
            || (size as usize) < std::mem::size_of::<WLAN_CONNECTION_ATTRIBUTES>()
        {
            return None;
        }
        let connection = &*data.0.cast::<WLAN_CONNECTION_ATTRIBUTES>();
        if connection.isState != wlan_interface_state_connected
            || connection.wlanConnectionMode != wlan_connection_mode_profile
        {
            return None;
        }
        let name = profile_name(&connection.strProfileName).ok()?;
        validate_profile(&name).ok()?;
        Some((adapter, name))
    }
}
fn profile_network_names(xml: &windows_core::HSTRING) -> windows_core::Result<Vec<String>> {
    use windows::Data::Xml::Dom::{XmlDocument, XmlLoadSettings};
    let settings = XmlLoadSettings::new()?;
    settings.SetProhibitDtd(true)?;
    settings.SetResolveExternals(false)?;
    let document = XmlDocument::new()?;
    document.LoadXmlWithSettings(xml, &settings)?;
    let nodes = document.SelectNodes(
        &"/*[local-name()='WLANProfile']/*[local-name()='SSIDConfig']/*[local-name()='SSID']"
            .into(),
    )?;
    let mut names = Vec::new();
    for index in 0..nodes.Length()? {
        let node = nodes.Item(index)?;
        let hex = node
            .SelectSingleNode(&"*[local-name()='hex']".into())
            .and_then(|node| node.InnerText())
            .ok();
        let name = if let Some(hex) = hex.filter(|hex| !hex.is_empty()) {
            let hex = hex.to_string();
            if !hex.is_ascii() || !hex.len().is_multiple_of(2) {
                return Err(unreadable());
            }
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|index| u8::from_str_radix(&hex[index..index + 2], 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| unreadable())?;
            let Ok(name) = String::from_utf8(bytes) else {
                continue;
            };
            name
        } else {
            node.SelectSingleNode(&"*[local-name()='name']".into())?
                .InnerText()?
                .to_string()
        };
        if crate::settings::validate_network(&name).is_ok() {
            names.push(name);
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}
fn network_description(adapter: windows_core::GUID) -> windows_core::Result<Option<String>> {
    unsafe {
        let manager: INetworkListManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)?;
        let connections = manager.GetNetworkConnections()?;
        loop {
            let mut item = [None];
            let mut fetched = 0;
            connections.Next(&mut item, Some(&mut fetched))?;
            if fetched == 0 {
                return Ok(None);
            }
            let connection = item[0].take().ok_or_else(unreadable)?;
            if connection.GetAdapterId()? == adapter && connection.IsConnected()?.as_bool() {
                let value = connection.GetNetwork()?.GetDescription()?.to_string();
                if !value.is_empty() {
                    return Ok(Some(value));
                }
            }
        }
    }
}
fn read_saved_networks(handle: HANDLE) -> Result<SavedNetworks, String> {
    let read = || -> windows_core::Result<SavedNetworks> {
        unsafe {
            let mut interfaces = WlanMemory(std::ptr::null_mut());
            let code = WlanEnumInterfaces(handle, None, &mut interfaces.0);
            if code != 0 || interfaces.0.is_null() {
                return Err(unreadable());
            }
            let interfaces = std::slice::from_raw_parts(
                (*interfaces.0).InterfaceInfo.as_ptr(),
                (*interfaces.0).dwNumberOfItems as usize,
            );
            let mut catalog = SavedNetworks::default();
            let mut current = Vec::new();
            let can_query = current_access_allowed();
            let mut reliable = true;
            for interface in interfaces {
                let mut list = WlanMemory(std::ptr::null_mut());
                let code = WlanGetProfileList(handle, &interface.InterfaceGuid, None, &mut list.0);
                if code != 0 || list.0.is_null() {
                    return Err(unreadable());
                }
                let profiles = std::slice::from_raw_parts(
                    (*list.0).ProfileInfo.as_ptr(),
                    (*list.0).dwNumberOfItems as usize,
                );
                let mut local = SavedNetworks::default();
                for profile in profiles {
                    let profile =
                        profile_name(&profile.strProfileName).map_err(|_| unreadable())?;
                    let mut xml = windows_core::PWSTR::null();
                    // Flags stay zero: only SSID metadata is retained; never request a plaintext key.
                    let mut flags = 0;
                    let code = WlanGetProfile(
                        handle,
                        &interface.InterfaceGuid,
                        &windows_core::HSTRING::from(&profile),
                        None,
                        &mut xml,
                        Some(&mut flags),
                        None,
                    );
                    let xml = WlanMemory(xml.0);
                    if code != 0 || xml.0.is_null() {
                        return Err(unreadable());
                    }
                    let metadata =
                        windows_core::HSTRING::from_wide(windows_core::PWSTR(xml.0).as_wide());
                    for name in profile_network_names(&metadata)? {
                        local.networks.push(SavedNetwork {
                            name,
                            profile: profile.clone(),
                        });
                    }
                }
                if interface.isState == wlan_interface_state_connected {
                    let description = network_description(interface.InterfaceGuid).ok().flatten();
                    // Unique same-adapter SSID metadata needs no Location permission.
                    let resolved = local.current_for(description.as_deref(), None).or_else(|| {
                        let profile = current_profile(handle, interface.InterfaceGuid, can_query)
                            .filter(|(source, _)| *source == interface.InterfaceGuid);
                        local.current_for(
                            description.as_deref(),
                            profile.as_ref().map(|(_, profile)| profile.as_str()),
                        )
                    });
                    if let Some(network) = resolved {
                        current.push(network);
                    } else {
                        reliable = false;
                    }
                } else if ![
                    wlan_interface_state_disconnected,
                    wlan_interface_state_not_ready,
                ]
                .contains(&interface.isState)
                {
                    reliable = false;
                }
                catalog.networks.extend(local.networks);
            }
            current.dedup();
            catalog.current = if reliable && current.len() == 1 {
                current.pop()
            } else {
                None
            };
            Ok(catalog)
        }
    };
    read().map_err(|_| "networksUnavailable".into())
} // The worker owns the COM apartment; these observations do not require Location.
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
            let name = network_description(adapter.InterfaceGuid)?;
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
    fn saved_metadata_extracts_ssids_without_displaying_profile_aliases() {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
            .ok()
            .unwrap();
        }
        let xml = r#"<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1"><name>Hidden alias</name><SSIDConfig><SSID><hex>486f6d652057692d4669</hex><name>Not the SSID</name></SSID><SSID><hex>e5aeb6e9878c</hex></SSID><SSID><name>Studio &amp; Guest</name></SSID></SSIDConfig></WLANProfile>"#;
        assert_eq!(
            profile_network_names(&xml.into()).unwrap(),
            vec!["Home Wi-Fi", "Studio & Guest", "家里"]
        );
        assert!(profile_network_names(
            &"<!DOCTYPE foo [<!ENTITY x SYSTEM 'file:///unreadable'>]><WLANProfile/>".into()
        )
        .is_err());
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
    #[test]
    #[ignore = "Read-only saved-profile enumeration; never connects or requests permission"]
    fn read_only_saved_networks() {
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
        let saved = wifi.saved_networks().unwrap();
        println!(
            "Saved-network metadata API available; count={}, current identity available={}",
            saved.networks.len(),
            saved.current.is_some()
        );
        if let Some(current) = saved.current {
            assert!(saved.networks.contains(&current));
        }
        drop(wifi);
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
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
