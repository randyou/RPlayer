fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icon.ico");
    resource.set("ProductName", "RPlayer");
    resource.set("FileDescription", "RPlayer");
    let cargo_version =
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let base_version = cargo_version
        .split(['-', '+'])
        .next()
        .unwrap_or("0.0.0");
    let file_version = format!("{base_version}.0");
    resource.set("FileVersion", file_version.as_str());
    resource.set("ProductVersion", file_version.as_str());
    resource.set("LegalCopyright", "MIT");
    if let Err(error) = resource.compile() {
        panic!("failed to embed Windows icon/version resource: {error}");
    }
}
