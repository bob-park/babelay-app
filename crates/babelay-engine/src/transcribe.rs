//! Whisper 전사기. GPU(Metal/CUDA) 우선, 실패하면 CPU로 폴백한다.
use std::path::Path;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

/// 16 kHz 기준 최소 입력 길이. whisper.cpp는 1초보다 짧은 버퍼를 거부한다.
const MIN_SAMPLES: usize = 16_000;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Segment {
    pub text: String,
    pub lang: String,
    pub t0_ms: u64,
    pub t1_ms: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum TranscribeError {
    #[error("model load failed: {0}")]
    Load(String),
    #[error("inference failed: {0}")]
    Inference(String),
}

pub trait Transcriber: Send {
    fn transcribe(
        &mut self,
        pcm16k: &[f32],
        lang: Option<&str>,
    ) -> Result<Vec<Segment>, TranscribeError>;
}

pub struct WhisperTranscriber {
    state: WhisperState,
    threads: i32,
    pub gpu_active: bool,
}

impl WhisperTranscriber {
    /// GPU 요청이 실패하면 CPU로 한 번 더 시도한다. 두 번째 반환값이 폴백 여부.
    pub fn load(model: &Path, use_gpu: bool) -> Result<(Self, bool), TranscribeError> {
        let make = |gpu: bool| {
            let mut p = WhisperContextParameters::default();
            p.use_gpu(gpu);
            WhisperContext::new_with_params(model, p)
        };
        let (ctx, fell_back) = match make(use_gpu) {
            Ok(c) => (c, false),
            Err(e) if use_gpu => (
                make(false).map_err(|e2| TranscribeError::Load(format!("{e}; cpu: {e2}")))?,
                true,
            ),
            Err(e) => return Err(TranscribeError::Load(e.to_string())),
        };
        let state = ctx
            .create_state()
            .map_err(|e| TranscribeError::Load(e.to_string()))?;
        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        let gpu_built = cfg!(any(feature = "metal", feature = "cuda"));
        Ok((
            Self {
                state,
                threads,
                gpu_active: use_gpu && !fell_back && gpu_built,
            },
            fell_back,
        ))
    }
}

pub(crate) fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Transcriber for WhisperTranscriber {
    fn transcribe(
        &mut self,
        pcm16k: &[f32],
        lang: Option<&str>,
    ) -> Result<Vec<Segment>, TranscribeError> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(lang.unwrap_or("auto")));
        params.set_n_threads(self.threads);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);

        // 1초 미만이면 0으로 채워 준다 (whisper.cpp가 짧은 버퍼에서 실패한다).
        let padded;
        let pcm = if pcm16k.len() < MIN_SAMPLES {
            padded = {
                let mut v = pcm16k.to_vec();
                v.resize(MIN_SAMPLES, 0.0);
                v
            };
            &padded[..]
        } else {
            pcm16k
        };

        self.state
            .full(params, pcm)
            .map_err(|e| TranscribeError::Inference(e.to_string()))?;

        let parts: Vec<String> = self
            .state
            .as_iter()
            .filter_map(|seg| seg.to_str_lossy().ok().map(|s| s.into_owned()))
            .collect();
        let text = join(&parts.iter().map(String::as_str).collect::<Vec<_>>());
        if text.is_empty() {
            return Ok(vec![]);
        }
        let lang = whisper_rs::get_lang_str(self.state.full_lang_id_from_state())
            .unwrap_or("en")
            .to_string();
        Ok(vec![Segment {
            text,
            lang,
            t0_ms: 0,
            t1_ms: 0,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_segments_skips_blank_and_trims() {
        assert_eq!(join(&["  Hello", "", " world. "]), "Hello world.");
    }

    #[test]
    #[ignore = "needs BABELAY_TEST_MODEL=path/to/ggml-tiny.bin"]
    fn transcribes_synthetic_silence_without_panicking() {
        let path = std::env::var("BABELAY_TEST_MODEL").expect("BABELAY_TEST_MODEL");
        let (mut t, fell_back) =
            WhisperTranscriber::load(std::path::Path::new(&path), true).unwrap();
        eprintln!("gpu_active={} fell_back={}", t.gpu_active, fell_back);
        let pcm = vec![0.0f32; 16_000];
        let segs = t.transcribe(&pcm, Some("en")).unwrap();
        assert!(segs.len() <= 1);
        // 짧은 입력도 패딩되어 패닉/에러 없이 지나가야 한다.
        assert!(t.transcribe(&[0.0f32; 100], Some("en")).is_ok());
    }
}
