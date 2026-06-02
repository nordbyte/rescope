#[cfg(feature = "tui")]
pub mod app;
#[cfg(feature = "tui")]
pub mod view;

#[cfg(not(feature = "tui"))]
pub fn is_available() -> bool {
    false
}

#[cfg(feature = "tui")]
pub fn is_available() -> bool {
    true
}
