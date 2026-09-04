//! WASAPI 루프백 캡처. 기본 출력 장치를 Capture 방향으로 열면 자동으로 루프백이 된다.

use super::{AudioSource, CaptureError, Frame, Sink};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode};

/// One open loopback stream. When the default device changes we rebuild the whole thing.
struct Stream {
    id: String,
    client: wasapi::AudioClient,
    event: wasapi::Handle,
    capture: wasapi::AudioCaptureClient,
    rate: u32,
    channels: u16,
}

fn default_render() -> Result<wasapi::Device, String> {
    DeviceEnumerator::new()
        .map_err(|e| e.to_string())?
        .get_default_device(&Direction::Render)
        .map_err(|e| e.to_string())
}

/// Opens the current default output device for loopback capture.
fn open() -> Result<Stream, String> {
    let device = default_render()?;
    let id = device.get_id().map_err(|e| e.to_string())?;
    let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
    let mix = client.get_mixformat().map_err(|e| e.to_string())?;
    let rate = mix.get_samplespersec();
    let channels = mix.get_nchannels();
    // The read loop below decodes 4 bytes as one little-endian f32. We no longer force
    // that format on the client, so reject a mix format it would misread.
    if mix.get_bitspersample() != 32 || !matches!(mix.get_subformat(), Ok(SampleType::Float)) {
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
    Ok(Stream {
        id,
        client,
        event,
        capture,
        rate,
        channels,
    })
}

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
            if let Err(e) = wasapi::initialize_mta().ok().map_err(|e| e.to_string()) {
                let _ = ready_tx.send(Err(e));
                return;
            }
            let mut stream = match open() {
                Ok(s) => s,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(()));

            let mut bytes: VecDeque<u8> = VecDeque::new();
            while !stop2.load(Ordering::Relaxed) {
                // An idle loopback endpoint stops signalling, and wait_for_event reports that
                // timeout as an error, so a timeout is normal: back off, then check whether the
                // default device moved out from under us.
                // ponytail: a truly failed handle spins at ~100 Hz until stop; fix = call
                // WaitForSingleObject directly and branch on WAIT_FAILED.
                let changed = if stream.event.wait_for_event(1000).is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    default_render()
                        .and_then(|d| d.get_id().map_err(|e| e.to_string()))
                        .is_ok_and(|id| id != stream.id)
                } else {
                    false
                };
                let read = if changed {
                    Err("default device changed".to_string())
                } else {
                    stream
                        .capture
                        .read_from_device_to_deque(&mut bytes)
                        .map_err(|e| e.to_string())
                };
                let info = match read {
                    Ok(info) => info,
                    Err(e) => {
                        // Device removed (AUDCLNT_E_DEVICE_INVALIDATED) or the default moved:
                        // reopen against whatever is now the default.
                        eprintln!("babelay capture: {e} — reopening the default device");
                        let _ = stream.client.stop_stream();
                        bytes.clear();
                        stream = loop {
                            if stop2.load(Ordering::Relaxed) {
                                return;
                            }
                            match open() {
                                Ok(s) => break s,
                                Err(e) => {
                                    eprintln!("babelay capture: reopen failed: {e}");
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                }
                            }
                        };
                        continue;
                    }
                };
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
                    rate: stream.rate,
                    channels: stream.channels,
                });
            }
            let _ = stream.client.stop_stream();
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
