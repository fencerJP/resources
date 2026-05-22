pub const APP_ID: &str = env!("APP_ID");
pub const GETTEXT_PACKAGE: &str = env!("GETTEXT_PACKAGE");
pub const LOCALEDIR: &str = env!("LOCALEDIR");
pub const PKGDATADIR: &str = env!("PKGDATADIR");
pub const DEVLOPMENT_BUILD: bool = match env!("DEVLOPMENT_BUILD").as_bytes() {
    b"true" => true,
    _ => false
};
pub const RESOURCES_FILE: &str = env!("RESOURCES_FILE");
pub const VERSION: &str = env!("VERSION");
pub const LIBEXECDIR: &str = env!("LIBEXECDIR");
