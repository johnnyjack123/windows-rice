//! Settings shared by every webview window.

/// Browser arguments of every webview. All windows share one WebView2
/// browser process, which only works with identical arguments, so every
/// builder must use this constant: wry's defaults plus autoplay without a
/// user gesture (alarm and pomodoro sounds).
pub const BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

/// Window title (the app name; not shown for the island or break windows).
pub const TITLE: &str = "rice";
