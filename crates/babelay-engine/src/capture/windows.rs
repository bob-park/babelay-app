//! WASAPI 루프백 캡처. 기본 출력 장치를 Capture 방향으로 열면 자동으로 루프백이 된다.

use super::{AudioSource, CaptureError, Frame, Sink};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

#[derive(Default)]
pub struct LoopbackSource {
    stop: Option<Arc<AtomicBool>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioSource for LoopbackSource {
    fn start(&mut self, mut sink: Sink) -> Result<(), CaptureError> {
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
                let fmt = WaveFormat::new(
                    32,
                    32,
                    &SampleType::Float,
                    rate as usize,
                    channels as usize,
                    None,
                );
                client
                    .initialize_client(
                        &fmt,
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
                while !stop2.load(Ordering::Relaxed) {
                    if event.wait_for_event(1000).is_err() {
                        continue;
                    }
                    capture
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
                let _ = ready_tx.send(Err(e));
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
