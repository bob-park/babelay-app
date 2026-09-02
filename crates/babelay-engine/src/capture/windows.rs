//! WASAPI 루프백 캡처. 구현은 Task 3.
use super::{AudioSource, CaptureError, Sink};

#[derive(Default)]
pub struct LoopbackSource;

impl AudioSource for LoopbackSource {
    fn start(&mut self, _sink: Sink) -> Result<(), CaptureError> {
        Err(CaptureError::Other("not implemented".into()))
    }
    fn stop(&mut self) {}
}
