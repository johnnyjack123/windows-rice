//! Every command of `web/src/backend/contract.ts`. Commands with input take
//! one parameter named `args`; all are async so they never block the UI thread.

mod app;
mod island;
mod pending;
mod system;

/// Registers commands and records their names for the contract test.
macro_rules! commands {
    ($($module:ident :: $name:ident),* $(,)?) => {
        /// Names of all registered commands.
        #[cfg_attr(not(test), allow(dead_code))]
        pub const NAMES: &[&str] = &[$(stringify!($name)),*];

        /// The invoke handler for `tauri::Builder::invoke_handler`.
        pub fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
            tauri::generate_handler![$($module::$name),*]
        }
    };
}

commands![
    app::app_info,
    app::config_get,
    app::config_set,
    app::theme_list_user,
    app::theme_save_user,
    app::theme_delete_user,
    app::window_open_settings,
    app::break_show,
    app::break_hide,
    app::break_state,
    app::break_finish,
    app::shell_open,
    system::system_appearance,
    system::system_wallpaper,
    system::system_fonts,
    system::system_monitors,
    island::island_configure,
    island::island_set_hit_regions,
    island::island_set_expanded,
    island::island_occlusion,
    pending::media_get,
    pending::media_control,
    pending::media_seek,
    pending::audio_get,
    pending::audio_set_volume,
    pending::audio_set_muted,
    pending::audio_set_device,
    pending::audio_set_mic_muted,
    pending::display_get,
    pending::display_set_brightness,
    pending::display_set_night_light,
    pending::power_get,
    pending::power_set_energy_saver,
    pending::power_action,
    pending::network_get,
    pending::network_scan,
    pending::network_set_wifi,
    pending::network_connect,
    pending::network_disconnect,
    pending::network_set_airplane,
    pending::network_set_hotspot,
    pending::bluetooth_get,
    pending::bluetooth_set_enabled,
    pending::bluetooth_connect,
    pending::bluetooth_disconnect,
    pending::notifications_get,
    pending::notifications_request_access,
    pending::notifications_dismiss,
    pending::notifications_clear,
    pending::calendar_accounts,
    pending::calendar_add_account,
    pending::calendar_remove_account,
    pending::calendar_set_enabled,
    pending::calendar_sync,
    pending::calendar_events,
    pending::tasks_list,
    pending::tasks_create,
    pending::tasks_update,
    pending::tasks_delete,
    pending::system_subscribe,
    pending::system_unsubscribe,
    pending::system_processes,
    pending::system_kill,
];

#[cfg(test)]
mod tests {
    use super::NAMES;
    use rice_core::contract::events;
    use std::collections::BTreeSet;

    fn contract() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../web/src/backend/contract.ts");
        std::fs::read_to_string(path).expect("contract.ts is readable")
    }

    /// Keys of a top-level `export interface <name> { … }` block in contract.ts.
    fn interface_keys(source: &str, name: &str) -> BTreeSet<String> {
        let start = source.find(&format!("export interface {name} {{")).expect("interface exists");
        let body = &source[start..];
        let end = body.find("\n}").expect("interface ends");
        body[..end]
            .lines()
            .skip(1)
            .filter_map(|line| {
                let line = line.trim();
                // Event names are quoted and contain "://" themselves.
                let key = match line.strip_prefix('\'') {
                    Some(quoted) => quoted.split('\'').next()?,
                    None => line.split(':').next()?.trim(),
                };
                let is_member = !line.starts_with('/') && !line.starts_with('*') && line.contains(':');
                (is_member && !key.is_empty() && !key.contains(' ')).then(|| key.to_owned())
            })
            .collect()
    }

    #[test]
    fn registers_exactly_the_contract_commands() {
        let expected = interface_keys(&contract(), "Commands");
        let registered: BTreeSet<String> = NAMES.iter().map(|n| (*n).to_owned()).collect();
        assert_eq!(registered.len(), NAMES.len(), "duplicate command");
        assert_eq!(registered, expected);
    }

    #[test]
    fn event_names_exist_in_the_contract() {
        let declared = interface_keys(&contract(), "Events");
        let used = [
            events::CONFIG_CHANGED,
            events::THEMES_CHANGED,
            events::APPEARANCE,
            events::WALLPAPER,
            events::ISLAND_OCCLUSION,
            events::ISLAND_EDGE,
            events::ISLAND_FOCUS_LOST,
            events::ISLAND_TOGGLE,
            events::BREAK_STATE,
            events::BREAK_FINISHED,
        ];
        for name in used {
            assert!(declared.contains(name), "{name} is not in contract.ts");
        }
    }
}
