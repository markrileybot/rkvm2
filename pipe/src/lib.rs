#[cfg(target_os = "linux")]
pub use linux::pipe;
#[cfg(target_os = "windows")]
pub use windows::pipe;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

