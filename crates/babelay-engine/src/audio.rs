//! 16kHz 모노 변환과 음성 조각화.
pub const TARGET_RATE: u32 = 16_000;
const FRAME: usize = TARGET_RATE as usize / 50; // 20ms
const RMS_THRESHOLD: f32 = 0.01;
const SILENCE_END: usize = TARGET_RATE as usize * 6 / 10; // 0.6s
const MAX_CHUNK: usize = TARGET_RATE as usize * 8;
const PARTIAL_EVERY: usize = TARGET_RATE as usize * 2;
const DROP_SILENCE: usize = TARGET_RATE as usize; // 1s

/// 인터리브 입력을 모노 16kHz 로 바꾸는 선형 보간 리샘플러.
pub struct Resampler {
    src_rate: u32,
    channels: u16,
    pos: f64,  // 다음 출력 샘플의 소스 위치(모노 샘플 단위)
    last: f32, // 직전 소스 모노 샘플(경계 보간용)
    have_last: bool,
}

impl Resampler {
    pub fn new(src_rate: u32, channels: u16) -> Self {
        Self {
            src_rate,
            channels: channels.max(1),
            pos: 0.0,
            last: 0.0,
            have_last: false,
        }
    }

    /// 인터리브 입력을 모노 16kHz 로 변환해 `out` 에 덧붙인다.
    pub fn push(&mut self, interleaved: &[f32], out: &mut Vec<f32>) {
        let ch = self.channels as usize;
        let mono: Vec<f32> = interleaved
            .chunks_exact(ch)
            .map(|f| f.iter().sum::<f32>() / ch as f32)
            .collect();
        if mono.is_empty() {
            return;
        }
        let step = self.src_rate as f64 / TARGET_RATE as f64;
        // 소스 인덱스 -1 은 직전 블록의 마지막 샘플
        let at = |i: i64, last: f32, mono: &[f32]| -> f32 {
            if i < 0 {
                last
            } else {
                mono[i as usize]
            }
        };
        let base = if self.have_last { -1.0 } else { 0.0 };
        let mut pos = self.pos + base; // 이번 블록 좌표계(-1 = last)
        let n = mono.len() as f64;
        while pos + 1.0 < n {
            let i = pos.floor() as i64;
            let frac = (pos - pos.floor()) as f32;
            let a = at(i, self.last, &mono);
            let b = at(i + 1, self.last, &mono);
            out.push(a + (b - a) * frac);
            pos += step;
        }
        self.pos = pos - (n - 1.0); // 다음 블록에서 last 가 -1 이 되도록
        self.last = *mono.last().unwrap();
        self.have_last = true;
    }
}

/// 청커가 내보내는 조각.
pub enum ChunkEvent {
    Partial {
        pcm: Vec<f32>,
        start_ms: u64,
    },
    Final {
        pcm: Vec<f32>,
        start_ms: u64,
        end_ms: u64,
    },
}

/// 20ms 프레임 RMS 로 음성/무음을 가르는 에너지 VAD 청커.
pub struct Chunker {
    buf: Vec<f32>,
    consumed: u64, // 세션 시작부터 버퍼 앞까지 흘려보낸 샘플 수
    speech_seen: bool,
    silence_run: usize,
    since_partial: usize,
    pending: Vec<f32>, // 20ms 프레임 미만 잔여
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            consumed: 0,
            speech_seen: false,
            silence_run: 0,
            since_partial: 0,
            pending: Vec::new(),
        }
    }

    fn ms(samples: u64) -> u64 {
        samples * 1000 / TARGET_RATE as u64
    }

    pub fn push(&mut self, mono16k: &[f32]) -> Vec<ChunkEvent> {
        let mut events = Vec::new();
        let mut data = std::mem::take(&mut self.pending);
        data.extend_from_slice(mono16k);
        let (frames, remainder) = data.as_chunks::<FRAME>();
        for frame in frames {
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / FRAME as f32).sqrt();
            let speech = rms > RMS_THRESHOLD;
            self.buf.extend_from_slice(frame);
            if speech {
                self.speech_seen = true;
                self.silence_run = 0;
            } else {
                self.silence_run += FRAME;
            }
            if !self.speech_seen {
                if self.buf.len() >= DROP_SILENCE {
                    self.consumed += self.buf.len() as u64;
                    self.buf.clear();
                    self.silence_run = 0;
                    self.since_partial = 0;
                }
                continue;
            }
            // 무음만 쌓이는 동안은 partial 타이머를 돌리지 않는다.
            self.since_partial += FRAME;
            if self.silence_run >= SILENCE_END || self.buf.len() >= MAX_CHUNK {
                events.push(self.finalize());
            } else if self.since_partial >= PARTIAL_EVERY {
                self.since_partial = 0;
                events.push(ChunkEvent::Partial {
                    pcm: self.buf.clone(),
                    start_ms: Self::ms(self.consumed),
                });
            }
        }
        self.pending = remainder.to_vec();
        events
    }

    fn finalize(&mut self) -> ChunkEvent {
        let pcm = std::mem::take(&mut self.buf);
        let start_ms = Self::ms(self.consumed);
        self.consumed += pcm.len() as u64;
        let end_ms = Self::ms(self.consumed);
        self.speech_seen = false;
        self.silence_run = 0;
        self.since_partial = 0;
        ChunkEvent::Final {
            pcm,
            start_ms,
            end_ms,
        }
    }

    pub fn flush(&mut self) -> Option<ChunkEvent> {
        if self.speech_seen && !self.buf.is_empty() {
            Some(self.finalize())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, secs: f32, amp: f32) -> Vec<f32> {
        (0..(rate as f32 * secs) as usize)
            .map(|i| amp * (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin())
            .collect()
    }
    fn silence(secs: f32) -> Vec<f32> {
        vec![0.0; (TARGET_RATE as f32 * secs) as usize]
    }

    #[test]
    fn resampler_downmixes_and_halves_48k_to_16k() {
        let mut r = Resampler::new(48_000, 2);
        let stereo: Vec<f32> = (0..48_000 * 2)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let mut out = Vec::new();
        r.push(&stereo, &mut out);
        assert!((out.len() as i64 - 16_000).abs() <= 2, "got {}", out.len());
        assert!(
            out.iter().all(|s| s.abs() < 1e-6),
            "stereo average of ±0.5 must be 0"
        );
    }

    #[test]
    fn resampler_44100_produces_about_16000_per_second() {
        let mut r = Resampler::new(44_100, 1);
        let mut out = Vec::new();
        r.push(&sine(44_100, 1.0, 0.5), &mut out);
        assert!((out.len() as i64 - 16_000).abs() <= 2, "got {}", out.len());
    }

    #[test]
    fn resampler_is_block_size_independent() {
        let src = sine(44_100, 3.0, 0.5);
        let resample = |blk: usize| {
            let mut r = Resampler::new(44_100, 1);
            let mut out = Vec::new();
            for c in src.chunks(blk) {
                r.push(c, &mut out);
            }
            out
        };
        let whole = resample(src.len());
        for blk in [441, 1000] {
            let blocked = resample(blk);
            assert!(
                (whole.len() as i64 - blocked.len() as i64).abs() <= 1,
                "blk {blk}: {} vs whole {}",
                blocked.len(),
                whole.len()
            );
            for (i, (w, b)) in whole.iter().zip(blocked.iter()).enumerate() {
                assert!(
                    (w - b).abs() < 1e-4,
                    "blk {blk} sample {i}: {b} vs whole {w}"
                );
            }
        }
    }

    #[test]
    fn chunker_finalizes_after_silence() {
        let mut c = Chunker::new();
        let mut ev = c.push(&sine(TARGET_RATE, 1.0, 0.3));
        ev.extend(c.push(&silence(0.7)));
        let finals: Vec<_> = ev
            .iter()
            .filter(|e| matches!(e, ChunkEvent::Final { .. }))
            .collect();
        assert_eq!(finals.len(), 1);
        if let ChunkEvent::Final {
            pcm,
            start_ms,
            end_ms,
        } = finals[0]
        {
            assert_eq!(*start_ms, 0);
            assert!(*end_ms >= 1000 && *end_ms <= 1700, "end_ms {end_ms}");
            assert!(pcm.len() >= TARGET_RATE as usize);
        }
    }

    #[test]
    fn chunker_emits_partials_every_two_seconds_and_caps_at_eight() {
        let mut c = Chunker::new();
        let ev = c.push(&sine(TARGET_RATE, 9.0, 0.3));
        let partials = ev
            .iter()
            .filter(|e| matches!(e, ChunkEvent::Partial { .. }))
            .count();
        let finals = ev
            .iter()
            .filter(|e| matches!(e, ChunkEvent::Final { .. }))
            .count();
        assert!(partials >= 3, "partials {partials}");
        assert_eq!(finals, 1, "8s cap must produce exactly one Final");
    }

    #[test]
    fn chunker_drops_leading_silence_and_tracks_offsets() {
        let mut c = Chunker::new();
        let mut ev = c.push(&silence(2.0));
        assert!(ev.is_empty());
        ev.extend(c.push(&sine(TARGET_RATE, 0.5, 0.3)));
        ev.extend(c.push(&silence(0.7)));
        let f = ev
            .iter()
            .find(|e| matches!(e, ChunkEvent::Final { .. }))
            .expect("final");
        if let ChunkEvent::Final { start_ms, .. } = f {
            assert!((1900..=2100).contains(start_ms), "start_ms {start_ms}");
        }
    }

    #[test]
    fn flush_returns_pending_speech() {
        let mut c = Chunker::new();
        c.push(&sine(TARGET_RATE, 0.5, 0.3));
        assert!(matches!(c.flush(), Some(ChunkEvent::Final { .. })));
        assert!(c.flush().is_none());
    }
}
