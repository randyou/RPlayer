fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icon.ico");
    resource.set("ProductName", "RPlayer");
    resource.set("FileDescription", "RPlayer");
    resource.set("FileVersion", "0.10.15.0");
    resource.set("ProductVersion", "0.10.15.0");
    resource.set("LegalCopyright", "MIT");
    if let Err(error) = resource.compile() {
        panic!("failed to embed Windows icon/version resource: {error}");
    }
}
