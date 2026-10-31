use homing::settings::{validate_profile, Settings};
#[test]
fn settings_roundtrip_preserves_unicode_spaces_and_delimiters() {
    let settings = Settings {
        profile: " 家里的 Wi-Fi = [2]; & \" ".into(),
        network: "家里的 Wi-Fi".into(),
        language: "zh".into(),
        pause_minutes: 45,
    };
    assert_eq!(Settings::parse(&settings.encode()).unwrap(), settings);
    assert_eq!(
        Settings::parse("[Settings]\nPauseMinutes=720\n")
            .unwrap()
            .pause_minutes,
        720
    );
}
#[test]
fn malformed_settings_fall_back_without_overwriting_the_original() {
    for input in [
        "[Settings",
        "garbage",
        "[Settings]\nProfileHex=zz",
        "[Settings]\nProfileHex=ff",
        "[Settings]\nProfileHex=e4b8",
        "[Settings]\nProfileHex=00",
        "[Settings]\nProfileHex=20",
        "[Settings]\nPauseMinutes=0",
        "[Settings]\nPauseMinutes=721",
        "[Settings]\nPauseMinutes=1.5",
        "[Settings]\nLanguage=fr",
    ] {
        assert!(Settings::parse(input).is_err(), "{input}");
    }
    let directory = std::env::temp_dir().join(format!(
        "homing-settings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("Homing.ini");
    let result = std::panic::catch_unwind(|| {
        assert_eq!(Settings::load(&path), (Settings::default(), None));
        std::fs::write(&path, "broken").unwrap();
        let (settings, error) = Settings::load(&path);
        assert_eq!(settings, Settings::default());
        assert!(error.is_some());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
        let settings = Settings {
            profile: "Home".into(),
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), (settings.clone(), None));
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), (settings, None));
        assert!(!path.with_extension("ini.tmp").exists());
    });
    std::fs::remove_dir_all(&directory).unwrap();
    result.unwrap();
}
#[test]
fn profile_boundary_rejects_controls_and_blank_names_but_never_interprets_shell_text() {
    for value in ["", " ", "\t", "Home\n", "Home\r", "Home\0"] {
        assert!(validate_profile(value).is_err());
    }
    assert!(validate_profile(&"a".repeat(256)).is_err());
    assert!(validate_profile("家 & \" %PATH% = [x]").is_ok());
}

#[test]
fn network_names_resolve_internal_aliases_and_failed_saves_preserve_settings() {
    use homing::{
        core::{Network, WifiBackend},
        networks::{SavedNetwork, SavedNetworks},
    };
    struct Simulated {
        catalog: Result<SavedNetworks, String>,
        queries: usize,
    }
    impl WifiBackend for Simulated {
        fn saved_networks(&mut self) -> Result<SavedNetworks, String> {
            self.queries += 1;
            self.catalog.clone()
        }
        fn inspect(&mut self) -> Result<Network, String> {
            panic!("selection must not inspect or connect")
        }
        fn connect(&mut self, _: &str) -> Result<(), String> {
            panic!("selection must not connect")
        }
    }
    let directory = std::env::temp_dir().join(format!("homing-target-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("Homing.ini");
    let result = std::panic::catch_unwind(|| {
        let home = SavedNetwork {
            name: "家里的网络".into(),
            profile: "Hidden alias 3".into(),
        };
        let studio = SavedNetwork {
            name: "Studio Wi-Fi".into(),
            profile: "Studio alias".into(),
        };
        let mut settings = Settings {
            profile: "Legacy substring".into(),
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut wifi = Simulated {
            catalog: Ok(SavedNetworks {
                networks: vec![home.clone(), studio.clone()],
                current: Some(home.clone()),
            }),
            queries: 0,
        };
        for value in [
            "",
            "Studio\n",
            "Hidden alias 3",
            "studio wi-fi",
            "Removed network",
            "a very long SSID exceeds thirty-two bytes",
        ] {
            assert!(settings.set_target(Some(value), &mut wifi, &path).is_err());
            assert_eq!(settings.profile, "Legacy substring");
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
        assert_eq!(
            wifi.queries, 3,
            "malformed SSIDs are rejected before lookup"
        );
        settings
            .set_target(Some("家里的网络"), &mut wifi, &path)
            .unwrap();
        assert_eq!(settings.profile, "Hidden alias 3");
        assert_eq!(settings.network, "家里的网络");
        settings
            .set_target(Some("Studio Wi-Fi"), &mut wifi, &path)
            .unwrap();
        assert_eq!(settings.profile, "Studio alias");
        settings.set_target(None, &mut wifi, &path).unwrap();
        assert_eq!(Settings::load(&path).0.network, "家里的网络");
        let saved = std::fs::read(&path).unwrap();
        wifi.catalog = Ok(SavedNetworks {
            networks: vec![home.clone()],
            current: None,
        });
        assert!(settings.set_target(None, &mut wifi, &path).is_err());
        wifi.catalog = Err("networksUnavailable".into());
        assert!(settings
            .set_target(Some("家里的网络"), &mut wifi, &path)
            .is_err());
        assert_eq!(settings.profile, "Hidden alias 3");
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        wifi.catalog = Ok(SavedNetworks {
            networks: vec![studio],
            current: None,
        });
        let before = settings.clone();
        assert!(settings
            .set_target(Some("Studio Wi-Fi"), &mut wifi, &directory)
            .is_err());
        assert_eq!(settings, before, "write failure preserves every setting");
    });
    std::fs::remove_dir_all(&directory).unwrap();
    result.unwrap();
}

#[test]
fn duplicate_ssids_require_reliable_identity_and_never_expose_profile_aliases() {
    use homing::networks::{SavedNetwork, SavedNetworks};
    let a = SavedNetwork {
        name: "Shared Wi-Fi".into(),
        profile: "Private alias A".into(),
    };
    let b = SavedNetwork {
        name: "Shared Wi-Fi".into(),
        profile: "Private alias B".into(),
    };
    let mut catalog = SavedNetworks {
        networks: vec![a.clone(), b.clone()],
        current: None,
    };
    assert_eq!(catalog.choices().names, vec!["Shared Wi-Fi"]);
    assert!(catalog.resolve("Shared Wi-Fi").is_err());
    assert!(catalog.current_for(Some("Shared Wi-Fi"), None).is_none());
    assert_eq!(
        catalog.current_for(Some("Shared Wi-Fi"), Some("Private alias B")),
        Some(b.clone())
    );
    assert!(catalog
        .current_for(Some("Other Wi-Fi"), Some("Unrelated alias"))
        .is_none());
    catalog.current = Some(b.clone());
    assert_eq!(catalog.resolve("Shared Wi-Fi").unwrap(), b);
    catalog.networks = vec![a.clone()];
    assert_eq!(catalog.current_for(Some("Shared Wi-Fi"), None), Some(a));
    let payload = serde_json::to_string(&catalog.choices()).unwrap();
    assert!(!payload.contains("Private alias"));
}
#[test]
fn current_mapping_uses_only_the_same_adapters_saved_metadata() {
    use homing::networks::{SavedNetwork, SavedNetworks};
    let a = SavedNetwork {
        name: "Network A".into(),
        profile: "Alias A".into(),
    };
    let local = SavedNetworks {
        networks: vec![a.clone()],
        current: None,
    };
    assert!(local.current_for(Some("Network B"), None).is_none());
    assert!(local
        .current_for(Some("Network A"), Some("Alias B"))
        .is_none());
    assert_eq!(local.current_for(Some("Network A"), None), Some(a));
    assert_eq!(
        local.name_for_profile("Alias A").as_deref(),
        Some("Network A")
    );
    assert_eq!(
        Settings::parse("[Settings]\nProfileHex=486f6d65\n")
            .unwrap()
            .network,
        ""
    );
}
