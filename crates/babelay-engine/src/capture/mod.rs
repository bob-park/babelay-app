//! 시스템 오디오 캡처. 플랫폼별 구현은 하위 모듈.

/// 인터리브된 f32 샘플 한 덩어리.
pub struct Frame {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub channels: u16,
}

pub type Sink = Box<dyn FnMut(Frame) + Send + 'static>;

pub trait AudioSource: Send {
    fn start(&mut self, sink: Sink) -> Result<(), CaptureError>;
    fn stop(&mut self);
}

#[derive(thiserror::Error, Debug)]
pub enum CaptureError {
    #[error("permission denied")]
    Permission,
    #[error("no output device")]
    NoDevice,
    #[error("os error {0}")]
    Os(i32),
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Granted,
    Denied,
    Unknown,
}

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn default_source() -> Box<dyn AudioSource> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::TapSource::default())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::LoopbackSource::default())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(Unsupported)
    }
}

pub fn probe_permission() -> Permission {
    #[cfg(target_os = "macos")]
    {
        macos::probe()
    }
    #[cfg(target_os = "windows")]
    {
        Permission::Granted
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Permission::Unknown
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
struct Unsupported;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl AudioSource for Unsupported {
    fn start(&mut self, _: Sink) -> Result<(), CaptureError> {
        Err(CaptureError::Other("unsupported platform".into()))
    }
    fn stop(&mut self) {}
}
