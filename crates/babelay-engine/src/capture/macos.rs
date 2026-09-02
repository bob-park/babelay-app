//! Core Audio Process Tap 기반 시스템 오디오 캡처(macOS 14.2+).
use super::{AudioSource, CaptureError, Frame, Permission, Sink};
use std::ffi::c_void;

type Cb = unsafe extern "C" fn(*const f32, u32, u32, f64, *mut c_void);

extern "C" {
    fn babelay_tap_start(cb: Cb, user: *mut c_void, handle_out: *mut *mut c_void) -> i32;
    fn babelay_tap_stop(handle: *mut c_void);
    fn babelay_tap_probe() -> i32;
}

pub struct TapSource {
    handle: *mut c_void,
    sink: Option<Box<Sink>>,
}

impl Default for TapSource {
    fn default() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            sink: None,
        }
    }
}

// 핸들은 ObjC 심이 소유하는 불투명 포인터. 한 번에 한 스레드만 만진다.
unsafe impl Send for TapSource {}

unsafe extern "C" fn trampoline(
    data: *const f32,
    frames: u32,
    channels: u32,
    rate: f64,
    user: *mut c_void,
) {
    let sink = &mut *(user as *mut Sink);
    let n = frames as usize * channels as usize;
    let samples = std::slice::from_raw_parts(data, n).to_vec();
    sink(Frame {
        samples,
        rate: rate as u32,
        channels: channels as u16,
    });
}

impl AudioSource for TapSource {
    fn start(&mut self, sink: Sink) -> Result<(), CaptureError> {
        self.stop();
        let user = Box::into_raw(Box::new(sink));
        let mut handle = std::ptr::null_mut();
        let st = unsafe { babelay_tap_start(trampoline, user as *mut c_void, &mut handle) };
        if st != 0 {
            unsafe { drop(Box::from_raw(user)) };
            // 탭 생성 자체가 막힌 경우(=권한 거부)와 그 외 OS 오류를 구분한다.
            return Err(if probe() == Permission::Denied {
                CaptureError::Permission
            } else {
                CaptureError::Os(st)
            });
        }
        self.handle = handle;
        // 소유권 회수: 콜백이 같은 힙 주소를 쓰므로 stop() 이 IOProc 을 멈춘 뒤에야 drop 된다.
        self.sink = Some(unsafe { Box::from_raw(user) });
        Ok(())
    }

    fn stop(&mut self) {
        if !self.handle.is_null() {
            unsafe { babelay_tap_stop(self.handle) };
            self.handle = std::ptr::null_mut();
        }
        self.sink = None;
    }
}

impl Drop for TapSource {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn probe() -> Permission {
    match unsafe { babelay_tap_probe() } {
        0 => Permission::Granted,
        1 => Permission::Denied,
        _ => Permission::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    #[ignore = "needs system audio permission; run with --ignored while audio plays"]
    fn captures_some_frames() {
        let got = Arc::new(Mutex::new(0usize));
        let g = got.clone();
        let mut src = TapSource::default();
        src.start(Box::new(move |f: Frame| {
            *g.lock().unwrap() += f.samples.len();
        }))
        .unwrap();
        std::thread::sleep(Duration::from_secs(1));
        src.stop();
        let n = *got.lock().unwrap();
        println!("captured {n} samples");
        assert!(n > 0);
    }
}
