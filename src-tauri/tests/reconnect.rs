use homing::{
    core::{Connection, Network, Reconnector, WifiBackend},
    settings::Settings,
};
#[derive(Default)]
struct Simulated {
    network: Network,
    requests: Vec<String>,
    fail: bool,
    query_fail: bool,
    notifications_unavailable: bool,
}
impl WifiBackend for Simulated {
    fn notification_error(&self) -> Option<String> {
        self.notifications_unavailable
            .then(|| "Notifications unavailable".into())
    }
    fn inspect(&mut self) -> Result<Network, String> {
        if self.query_fail {
            Err("Query failed".into())
        } else {
            Ok(self.network.clone())
        }
    }
    fn connect(&mut self, profile: &str) -> Result<(), String> {
        self.requests.push(profile.into());
        if self.fail {
            Err("Profile unavailable".into())
        } else {
            Ok(())
        }
    }
}
fn guard() -> Reconnector {
    Reconnector::new(Settings {
        profile: "Home".into(),
        ..Settings::default()
    })
}
#[test]
fn original_network_names_drive_display_and_literal_matching_without_profile_suffixes() {
    // NLM GetName/GetDescription pairs: the backend observes the description only.
    for (profile_label, actual, target, connected) in [
        ("Home", "Home", "Home", true),
        ("HomeNetwork_5 3", "HomeNetwork_5", "HomeNetwork_5", true),
        ("HomeNetwork_5 3", "HomeNetwork_5", "HomeNetwork_5 3", false),
        ("Studio 3 2", "Studio 3", "Studio 3", true),
        ("Studio 3 2", "Studio 3", "Studio 3 2", false),
    ] {
        let mut guard = Reconnector::new(Settings {
            profile: target.into(),
            ..Settings::default()
        });
        let mut wifi = Simulated {
            network: Network {
                current: Some(actual.into()),
                interface_text: actual.into(),
            },
            ..Default::default()
        };
        guard.reconcile(&mut wifi, || 0, 1000);
        assert_eq!(
            guard.state(0).current.as_deref(),
            Some(actual),
            "{profile_label}"
        );
        assert_eq!(
            guard.state(0).connection == Connection::Connected,
            connected,
            "{profile_label}"
        );
        assert_eq!(wifi.requests.is_empty(), connected, "{profile_label}");
    }
}
#[test]
fn legacy_case_sensitive_literal_substring_and_missing_profile_rules() {
    for (text, expected) in [
        ("SSID : Home", 0),
        ("Profile : MyHome5G", 0),
        ("SSID : home", 1),
        ("SSID : Away", 1),
        ("", 1),
    ] {
        let mut wifi = Simulated {
            network: Network {
                current: Some("Away".into()),
                interface_text: text.into(),
            },
            ..Default::default()
        };
        let mut guard = guard();
        guard.reconcile(&mut wifi, || 0, 1000);
        assert_eq!(wifi.requests.len(), expected, "{text}");
        assert_eq!(
            guard.state(0).connection,
            if expected == 0 {
                Connection::Connected
            } else {
                Connection::Reconnecting
            }
        );
    }
    let mut wifi = Simulated::default();
    let mut guard = Reconnector::new(Settings::default());
    guard.reconcile(&mut wifi, || 0, 1000);
    assert!(wifi.requests.is_empty());
    assert_eq!(guard.deadline(0), None);
}
#[test]
fn accepted_requests_verify_after_one_second_and_failed_requests_wait_sixty_one() {
    for (fail, deadline) in [(false, 1), (true, 61)] {
        let mut wifi = Simulated {
            fail,
            ..Default::default()
        };
        let mut guard = guard();
        guard.reconcile(&mut wifi, || 0, 1000);
        assert_eq!(guard.deadline(0), Some(deadline * 1000));
        guard.reconcile(&mut wifi, || 0, 1000);
        assert_eq!(
            wifi.requests.len(),
            1,
            "notifications must not bypass cooldown"
        );
        guard.reconcile(&mut wifi, || deadline * 1000, 1000 + deadline);
        assert_eq!(wifi.requests.len(), 2);
        wifi.network.interface_text = "SSID : Home".into();
        wifi.network.current = Some("Home".into());
        guard.reconcile(&mut wifi, || deadline * 1000, 1000 + deadline);
        assert_eq!(
            guard.state(deadline * 1000).connection,
            Connection::Connected
        );
        assert_eq!(
            guard.deadline(deadline * 1000),
            None,
            "connected state must not poll"
        );
        assert_eq!(
            guard.state(deadline * 1000).last_reconnect,
            if fail { None } else { Some(1000 + deadline) }
        );
    }
}
#[test]
fn pause_blocks_writes_expires_and_resume_reconciles_immediately() {
    let mut guard = guard();
    let mut wifi = Simulated::default();
    guard.pause(10_000);
    guard.reconcile(&mut wifi, || 10 * 1000, 1010);
    assert!(wifi.requests.is_empty());
    assert_eq!(guard.deadline(10_000), Some(3_610_000));
    guard.settings.pause_minutes = 15;
    guard.reconcile(&mut wifi, || 3609 * 1000, 4609);
    assert!(
        wifi.requests.is_empty(),
        "changing duration preserves the active deadline"
    );
    guard.reconcile(&mut wifi, || 3610 * 1000, 4610);
    assert_eq!(wifi.requests, vec!["Home"]);
    guard.pause(3_611_000);
    guard.reconcile(&mut wifi, || 3612 * 1000, 4612);
    assert_eq!(wifi.requests.len(), 1);
    guard.resume();
    guard.reconcile(&mut wifi, || 3612 * 1000, 4612);
    assert_eq!(wifi.requests.len(), 2);
}
#[test]
fn unreadable_state_never_connects_and_recovers_on_a_known_network() {
    let mut guard = guard();
    let mut wifi = Simulated {
        query_fail: true,
        fail: true,
        ..Default::default()
    };
    guard.reconcile(&mut wifi, || 0, 1000);
    assert!(wifi.requests.is_empty(), "query failure must never connect");
    assert!(guard.state(0).error.unwrap().contains("Query failed"));
    assert_eq!(guard.state(0).connection, Connection::Unreadable);
    assert_eq!(guard.state(0).last_reconnect, None);
    assert!(guard.deadline(0).is_some());
    guard.reconcile(&mut wifi, || 61_000, 1061);
    assert!(wifi.requests.is_empty());
    wifi.query_fail = false;
    wifi.network.current = Some("MyHome5G".into());
    wifi.network.interface_text = "MyHome5G".into();
    guard.reconcile(&mut wifi, || 62_000, 1062);
    assert_eq!(guard.state(62_000).connection, Connection::Connected);
    assert!(guard.state(62_000).error.is_none());
    assert!(wifi.requests.is_empty());
}
#[test]
fn retry_delay_starts_when_the_command_returns() {
    use std::cell::Cell;
    struct Slow<'a>(&'a Cell<u64>);
    impl WifiBackend for Slow<'_> {
        fn inspect(&mut self) -> Result<Network, String> {
            Ok(Network::default())
        }
        fn connect(&mut self, _: &str) -> Result<(), String> {
            self.0.set(2500);
            Ok(())
        }
    }
    let clock = Cell::new(0);
    let mut wifi = Slow(&clock);
    let mut guard = guard();
    guard.reconcile(&mut wifi, || clock.get(), 1000);
    assert_eq!(guard.deadline(clock.get()), Some(3500));
}
#[test]
fn unreadable_observation_does_not_shorten_a_failed_connect_cooldown() {
    let mut guard = guard();
    let mut wifi = Simulated {
        fail: true,
        ..Default::default()
    };
    guard.reconcile(&mut wifi, || 0, 1000);
    wifi.query_fail = true;
    guard.reconcile(&mut wifi, || 100, 1000);
    wifi.query_fail = false;
    guard.reconcile(&mut wifi, || 1100, 1001);
    assert_eq!(wifi.requests.len(), 1);
    assert_eq!(guard.deadline(1100), Some(61_000));
    assert_eq!(guard.state(1100).connection, Connection::Disconnected);
    assert!(guard.state(1100).error.is_none());
    guard.reconcile(&mut wifi, || 61_000, 1061);
    assert_eq!(wifi.requests.len(), 2);
}
#[test]
fn query_failure_clears_stale_network_and_blocks_pending_retries() {
    let mut guard = guard();
    let mut wifi = Simulated {
        network: Network {
            current: Some("Home".into()),
            interface_text: "Home".into(),
        },
        ..Default::default()
    };
    guard.reconcile(&mut wifi, || 0, 1000);
    wifi.query_fail = true;
    guard.reconcile(&mut wifi, || 1000, 1001);
    assert!(wifi.requests.is_empty());
    assert!(guard.state(1000).current.is_none());
    assert_eq!(guard.state(1000).connection, Connection::Unreadable);
    assert!(guard.state(1000).error.unwrap().contains("Query failed"));
}
#[test]
fn unavailable_notifications_use_legacy_observation_timer_and_stop_after_recovery() {
    let mut guard = guard();
    let mut wifi = Simulated {
        notifications_unavailable: true,
        network: Network {
            current: Some("Home".into()),
            interface_text: "SSID : Home".into(),
        },
        ..Default::default()
    };
    guard.reconcile(&mut wifi, || 0, 1000);
    assert!(wifi.requests.is_empty());
    assert_eq!(guard.deadline(0), Some(1000));
    assert!(guard
        .state(0)
        .error
        .unwrap()
        .contains("Notifications unavailable"));
    wifi.notifications_unavailable = false;
    guard.reconcile(&mut wifi, || 1000, 1001);
    assert_eq!(guard.deadline(1000), None);
    assert!(guard.state(1000).error.is_none());
    wifi.notifications_unavailable = true;
    wifi.network = Network::default();
    guard.pause(1000);
    guard.reconcile(&mut wifi, || 1000, 1001);
    assert!(wifi.requests.is_empty());
    assert_eq!(guard.deadline(1000), Some(2000));
    guard.reconcile(&mut wifi, || 3_601_000, 4601);
    assert_eq!(wifi.requests, vec!["Home"]);
    let mut unconfigured = Reconnector::new(Settings::default());
    wifi.requests.clear();
    unconfigured.reconcile(&mut wifi, || 0, 1000);
    assert!(wifi.requests.is_empty());
    assert_eq!(unconfigured.deadline(0), Some(61_000));
    wifi.notifications_unavailable = false;
    unconfigured.reconcile(&mut wifi, || 1000, 1001);
    assert_eq!(unconfigured.deadline(1000), None);
}

#[test]
fn system_network_name_matches_while_connection_requests_use_internal_profile_alias() {
    let mut guard = Reconnector::new(Settings {
        profile: "Hidden alias".into(),
        network: "Home Wi-Fi".into(),
        ..Settings::default()
    });
    let mut wifi = Simulated {
        network: Network {
            current: Some("Home Wi-Fi".into()),
            interface_text: "Home Wi-Fi".into(),
        },
        ..Default::default()
    };
    guard.reconcile(&mut wifi, || 0, 0);
    assert_eq!(guard.state(0).connection, Connection::Connected);
    assert!(wifi.requests.is_empty());
    assert_eq!(guard.state(0).target_name.as_deref(), Some("Home Wi-Fi"));
    let payload = serde_json::to_string(&guard.state(0)).unwrap();
    assert!(!payload.contains("Hidden alias"));
    wifi.network = Network {
        current: Some("Away Wi-Fi".into()),
        interface_text: "Away Wi-Fi".into(),
    };
    guard.reconcile(&mut wifi, || 1, 0);
    assert_eq!(wifi.requests, vec!["Hidden alias"]);
}
