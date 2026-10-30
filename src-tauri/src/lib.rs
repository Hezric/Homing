pub mod core;
#[cfg(windows)]
mod runtime;
pub mod settings;
#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod wifi;

#[cfg(windows)]
#[tauri::command]
fn snapshot(worker: tauri::State<'_, runtime::Worker>) -> Result<runtime::Snapshot, String> {
    worker.request(None)
}
#[cfg(windows)]
#[tauri::command]
fn update(
    action: runtime::Action,
    worker: tauri::State<'_, runtime::Worker>,
) -> Result<runtime::Snapshot, String> {
    worker.request(Some(action))
}

#[cfg(windows)]
pub fn run() {
    use tauri::Manager;
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            shell::open_flyout(app);
        }))
        .invoke_handler(tauri::generate_handler![
            snapshot,
            update,
            shell::resize_flyout,
            shell::present_flyout,
            shell::prepare_flyout,
            shell::dismiss_flyout,
            shell::open_project
        ])
        .setup(|app| {
            shell::install(app.handle())?;
            let path = std::env::var_os("APPDATA")
                .map(std::path::PathBuf::from)
                .ok_or("APPDATA is not available")?
                .join("Homing")
                .join("Homing.ini");
            app.manage(runtime::start(app.handle().clone(), path)?);
            Ok(())
        })
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|app, event| match event {
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            tauri::RunEvent::Exit => {
                if let Some(worker) = app.try_state::<runtime::Worker>() {
                    worker.stop();
                }
            }
            _ => {}
        }),
        Err(error) => {
            use windows::{
                core::HSTRING,
                Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
            };
            unsafe {
                MessageBoxW(
                    None,
                    &HSTRING::from(format!(
                        "Homing could not start. Reconnect is not running.\n{error}"
                    )),
                    &HSTRING::from("Homing"),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
    }
}
#[cfg(not(windows))]
pub fn run() {
    eprintln!("Homing requires Windows.");
}
