//! WASAPI 루프백 캡처. 기본 출력 장치를 Capture 방향으로 열면 자동으로 루프백이 된다.

use super::{AudioSource, CaptureError, Frame, Sink};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode};

#[derive(Default)]
pub struct LoopbackSource {
    stop: Option<Arc<AtomicBool>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioSource for LoopbackSource {
    fn start(&mut self, mut sink: Sink) -> Result<(), CaptureError> {
        self.stop();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        let thread = std::thread::spawn(move || {
            let mut run = || -> Result<(), String> {
                wasapi::initialize_mta().ok().map_err(|e| e.to_string())?;
                let device = DeviceEnumerator::new()
                    .map_err(|e| e.to_string())?
                    .get_default_device(&Direction::Render)
                    .map_err(|e| e.to_string())?;
                let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
                let mix = client.get_mixformat().map_err(|e| e.to_string())?;
                let rate = mix.get_samplespersec();
                let channels = mix.get_nchannels();
                // The read loop below decodes 4 bytes as one little-endian f32. We no longer force
                // that format on the client, so reject a mix format it would misread.
                if mix.get_bitspersample() != 32
                    || !matches!(mix.get_subformat(), Ok(SampleType::Float))
                {
                    return Err(format!(
                        "unsupported mix format: {} bit, {:?}",
                        mix.get_bitspersample(),
                        mix.get_subformat()
                    ));
                }
                client
                    .initialize_client(
                        &mix,
                        &Direction::Capture,
                        &StreamMode::EventsShared {
                            autoconvert: true,
                            buffer_duration_hns: 200_000,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                let event = client.set_get_eventhandle().map_err(|e| e.to_string())?;
                let capture = client.get_audiocaptureclient().map_err(|e| e.to_string())?;
                client.start_stream().map_err(|e| e.to_string())?;
                let _ = ready_tx.send(Ok(()));

                let mut bytes: VecDeque<u8> = VecDeque::new();
                // ponytail: wasapi's wait_for_event reports a plain timeout as an error
                // (api.rs:2029, WasapiError::EventTimeout), so a long silence looks the same as a
                // broken handle. Bail after 5 in a row rather than spin; if idle systems start
                // dropping capture, distinguish the two by checking WaitForSingleObject directly.
                let mut wait_failures = 0u32;
                while !stop2.load(Ordering::Relaxed) {
                    if event.wait_for_event(1000).is_err() {
                        wait_failures += 1;
                        if wait_failures >= 5 {
                            return Err("no audio event for 5 consecutive waits".into());
                        }
                        continue;
                    }
                    wait_failures = 0;
                    let info = capture
                        .read_from_device_to_deque(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    let n = bytes.len() / 4;
                    if n == 0 {
                        continue;
                    }
                    let mut samples = Vec::with_capacity(n);
                    for _ in 0..n {
                        let mut b = [0u8; 4];
                        for byte in b.iter_mut() {
                            *byte = bytes.pop_front().unwrap_or(0);
                        }
                        samples.push(f32::from_le_bytes(b));
                    }
                    if info.flags.silent {
                        samples.fill(0.0);
                    }
                    sink(Frame {
                        samples,
                        rate,
                        channels,
                    });
                }
                let _ = client.stop_stream();
                Ok(())
            };
            if let Err(e) = run() {
                // Before start() returns this reaches the caller; after, the receiver is gone
                // and the log is the only trace.
                if ready_tx.send(Err(e.clone())).is_err() {
                    eprintln!("babelay capture: {e}");
                }
            }
        });
        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.stop = Some(stop);
                self.thread = Some(thread);
                Ok(())
            }
            Ok(Err(e)) => Err(CaptureError::Other(e)),
            Err(_) => Err(CaptureError::Other("capture thread died".into())),
        }
    }

    fn stop(&mut self) {
        if let Some(s) = self.stop.take() {
            s.store(true, Ordering::Relaxed);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for LoopbackSource {
    fn drop(&mut self) {
        self.stop();
    }
}
