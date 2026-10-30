use homing::settings::{validate_profile, Settings};
#[test]
fn settings_roundtrip_preserves_unicode_spaces_and_delimiters() {
    let settings = Settings {
        profile: " 家里的 Wi-Fi = [2]; & \" ".into(),
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
