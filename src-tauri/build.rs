fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // The manifest links rice.exe to its sparse package (package identity for notifications).
    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    let manifest = include_str!("windows-app-manifest.xml");
    // Development builds get an identity of their own (scripts/sparse-package.ps1), so the
    // installed rice and `pnpm tauri dev` never take each other's away.
    let manifest = if std::env::var("PROFILE").as_deref() == Ok("release") {
        manifest.to_owned()
    } else {
        manifest.replace(r#"packageName="io.github.johnnyjack123.rice""#, r#"packageName="io.github.johnnyjack123.rice.dev""#)
    };
    let windows = tauri_build::WindowsAttributes::new().app_manifest(manifest);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))?;
    Ok(())
}
