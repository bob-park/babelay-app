//! Core Audio Process Tap 기반 시스템 오디오 캡처(macOS 14.2+).
use super::{AudioSource, CaptureError, Frame, Permission, Sink};
use std::ffi::c_void;

type Cb = unsafe extern "C" fn(*const f32, u32, u32, f64, *mut c_void);

/// 심이 macOS 14.2 미만에서 돌려주는 값.
const ERR_UNSUPPORTED_OS: i32 = -1;
/// 탭 포맷이 float32 가 아닐 때.
const ERR_BAD_FORMAT: i32 = -2;
/// `kAudioHardwareIllegalOperationError` ('nope'). TCC 허가가 없을 때 coreaudiod 가 돌려준다.
const ERR_NOPE: i32 = 0x6e6f_7065;

extern "C" {
    fn babelay_tap_start(cb: Cb, user: *mut c_void, handle_out: *mut *mut c_void) -> i32;
    fn babelay_tap_stop(handle: *mut c_void);
    fn babelay_tap_probe() -> i32;
}

pub struct TapSource {
    handle: *mut c_void,
    /// C 콜백이 그대로 들고 있는 포인터. `stop()` 에서 IOProc 을 멈춘 뒤에만 회수해 drop 한다.
    sink: *mut Sink,
}

impl Default for TapSource {
    fn default() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            sink: std::ptr::null_mut(),
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
    let n = frames as usize * channels as usize;
    if data.is_null() || n == 0 || user.is_null() {
        return;
    }
    let sink = &mut *(user as *mut Sink);
    let samples = std::slice::from_raw_parts(data, n).to_vec();
    let frame = Frame {
        samples,
        rate: rate as u32,
        channels: channels as u16,
    };
    // 패닉이 C 프레임을 넘어가면 UB. 여기서 잡아 버린다.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || sink(frame)));
}

impl AudioSource for TapSource {
    fn start(&mut self, sink: Sink) -> Result<(), CaptureError> {
        self.stop();
        let user = Box::into_raw(Box::new(sink));
        let mut handle = std::ptr::null_mut();
        let st = unsafe { babelay_tap_start(trampoline, user as *mut c_void, &mut handle) };
        if st != 0 {
            unsafe { drop(Box::from_raw(user)) };
            return Err(match st {
                ERR_UNSUPPORTED_OS => CaptureError::Other("macOS 14.2+ required".into()),
                ERR_BAD_FORMAT => CaptureError::Other("tap format is not float32".into()),
                ERR_NOPE => CaptureError::Permission,
                _ => CaptureError::Os(st),
            });
        }
        self.handle = handle;
        self.sink = user;
        Ok(())
    }

    fn stop(&mut self) {
        if !self.handle.is_null() {
            unsafe { babelay_tap_stop(self.handle) };
            self.handle = std::ptr::null_mut();
        }
        // IOProc 이 멈춘 뒤에야 sink 를 해제한다.
        if !self.sink.is_null() {
            unsafe { drop(Box::from_raw(self.sink)) };
            self.sink = std::ptr::null_mut();
        }
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
    fn trampoline_hands_the_sink_an_interleaved_frame() {
        let got: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
        let g = got.clone();
        let sink: Sink = Box::new(move |f: Frame| *g.lock().unwrap() = Some(f));
        let user = Box::into_raw(Box::new(sink));

        let data: [f32; 6] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
        unsafe { trampoline(data.as_ptr(), 3, 2, 48000.0, user as *mut c_void) };
        // 널 데이터·0 프레임은 무시되어야 한다.
        unsafe { trampoline(std::ptr::null(), 3, 2, 48000.0, user as *mut c_void) };
        unsafe { trampoline(data.as_ptr(), 0, 2, 48000.0, user as *mut c_void) };
        unsafe { drop(Box::from_raw(user)) };

        let f = got.lock().unwrap().take().expect("sink was called");
        assert_eq!(f.samples.len(), 6);
        assert_eq!(f.samples, data);
        assert_eq!(f.rate, 48000);
        assert_eq!(f.channels, 2);
    }

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
