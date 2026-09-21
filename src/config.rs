pub const APP_ID: &str = with_default(option_env!("APP_ID"), "org.gnome.Resources");
pub const GETTEXT_PACKAGE: &str = with_default(option_env!("GETTEXT_PACKAGE"), "resources");
pub const LOCALEDIR: &str = with_default(option_env!("LOCALEDIR"), "/usr/share/locale/");
pub const DEVLOPMENT_BUILD: bool =
    match with_default(option_env!("DEVLOPMENT_BUILD"), "false").as_bytes() {
        b"true" => true,
        b"false" => false,
        _ => panic!("Invalid bool value."),
    };
pub const RESOURCES_FILE: &str = with_default(
    option_env!("RESOURCES_FILE"),
    "/usr/share/resources/resources.gresource",
);
pub const VERSION: &str = with_default(option_env!("VERSION"), env!("CARGO_PKG_VERSION"));
pub const LIBEXECDIR: &str = with_default(option_env!("LIBEXECDIR"), "/usr/libexec");

const fn with_default(v: Option<&'static str>, default: &'static str) -> &'static str {
    match v {
        Some(v) => v,
        None => default,
    }
}
