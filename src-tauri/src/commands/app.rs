//! App info, settings, user themes, settings window, break windows, shell.

use crate::breaks;
use crate::config::{emit_themes, on_config_changed};
use crate::settings_window;
use crate::state::AppState;
use rice_core::contract::{
    AppInfo, BreakOverlay, ConfigSetArgs, OpenSettingsArgs, ShellOpenArgs, ThemeIdArgs, ThemeSaveArgs, UserTheme,
};
use rice_core::shell::{ShellTarget, classify, display_path, path_allowed};
use rice_core::{BackendError, BackendResult};
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> BackendResult<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        themes_dir: display_path(&state.paths.themes_dir),
        config_dir: display_path(&state.paths.config_dir),
    })
}

#[tauri::command]
pub async fn config_get(state: State<'_, AppState>) -> BackendResult<Value> {
    Ok(state.config.get())
}

#[tauri::command]
pub async fn config_set(app: AppHandle, state: State<'_, AppState>, args: ConfigSetArgs) -> BackendResult<Value> {
    let stored = state.config.set(&args.patch)?;
    on_config_changed(&app, &stored);
    Ok(stored)
}

#[tauri::command]
pub async fn theme_list_user(state: State<'_, AppState>) -> BackendResult<Vec<UserTheme>> {
    Ok(state.themes.list())
}

#[tauri::command]
pub async fn theme_save_user(app: AppHandle, state: State<'_, AppState>, args: ThemeSaveArgs) -> BackendResult<UserTheme> {
    let theme = state.themes.save(&args.id, &args.manifest, &args.css)?;
    emit_themes(&app);
    Ok(theme)
}

#[tauri::command]
pub async fn theme_delete_user(app: AppHandle, state: State<'_, AppState>, args: ThemeIdArgs) -> BackendResult<()> {
    state.themes.delete(&args.id)?;
    emit_themes(&app);
    Ok(())
}

#[tauri::command]
pub async fn window_open_settings(app: AppHandle, args: OpenSettingsArgs) -> BackendResult<()> {
    settings_window::open(&app, args.page.as_deref())
}

#[tauri::command]
pub async fn break_show(app: AppHandle, args: BreakOverlay) -> BackendResult<()> {
    breaks::show(&app, args)
}

#[tauri::command]
pub async fn break_hide(app: AppHandle) -> BackendResult<()> {
    breaks::hide(&app);
    Ok(())
}

#[tauri::command]
pub async fn break_state(state: State<'_, AppState>) -> BackendResult<Option<BreakOverlay>> {
    Ok(state.breaks.current())
}

#[tauri::command]
pub async fn break_finish(app: AppHandle) -> BackendResult<()> {
    breaks::finish(&app);
    Ok(())
}

/// Whitelist: `ms-settings:`, `https://`, and folders or text files inside the config folder.
#[tauri::command]
pub async fn shell_open(state: State<'_, AppState>, args: ShellOpenArgs) -> BackendResult<()> {
    let target = match classify(&args.target)? {
        ShellTarget::Uri(uri) => uri,
        ShellTarget::Path(path) => {
            let canonical = std::fs::canonicalize(&path).map_err(|_| BackendError::not_found(format!("{}", path.display())))?;
            let root = std::fs::canonicalize(&state.paths.config_dir)?;
            if !path_allowed(&canonical, &root, canonical.is_dir()) {
                return Err(BackendError::invalid_args("path is outside the rice folder or not a folder/text file"));
            }
            display_path(&canonical)
        }
    };
    tauri::async_runtime::spawn_blocking(move || rice_win::shell::open(&target))
        .await
        .map_err(BackendError::internal)?
        .map_err(BackendError::internal)
}
