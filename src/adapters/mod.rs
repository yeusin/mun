pub mod bookmarks;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub type CurrentPlatform = linux::LinuxPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacOSPlatform;
