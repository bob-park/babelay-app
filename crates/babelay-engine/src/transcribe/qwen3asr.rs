//! Qwen3-ASR 전사기(llama.cpp mtmd). 오디오 인코더(mmproj)가 낸 임베딩을 Qwen3 디코더에 넣고
//! greedy 로 `language X<asr_text>텍스트` 를 뽑는다.

use super::{Segment, TranscribeError, Transcriber, MIN_SAMPLES};
use crate::llama::backend;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::mtmd::{
    mtmd_default_marker, MtmdBitmap, MtmdContext, MtmdContextParams, MtmdInputText,
};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::Path;

/// 프롬프트(오디오 토큰 포함) + 생성분. 청크는 최대 8초라 오디오 토큰은 수백 개다.
const N_CTX: u32 = 2048;
/// 8초 발화는 100 토큰 안팎이다. 반복 폭주를 막는 상한.
const MAX_NEW: usize = 256;
/// mtmd 헬퍼가 임베딩을 디코더에 넣을 때의 배치 크기. 컨텍스트 기본 n_batch(2048) 이하.
const N_BATCH: i32 = 512;
const ASR_TAG: &str = "<asr_text>";

/// 모델이 내는 언어명 ↔ ISO 코드. Qwen3-ASR 카드의 30개 언어.
const LANGS: &[(&str, &str)] = &[
    ("Chinese", "zh"),
    ("English", "en"),
    ("Cantonese", "yue"),
    ("Arabic", "ar"),
    ("German", "de"),
    ("French", "fr"),
    ("Spanish", "es"),
    ("Portuguese", "pt"),
    ("Indonesian", "id"),
    ("Italian", "it"),
    ("Korean", "ko"),
    ("Russian", "ru"),
    ("Thai", "th"),
    ("Vietnamese", "vi"),
    ("Japanese", "ja"),
    ("Turkish", "tr"),
    ("Hindi", "hi"),
    ("Malay", "ms"),
    ("Dutch", "nl"),
    ("Swedish", "sv"),
    ("Danish", "da"),
    ("Finnish", "fi"),
    ("Polish", "pl"),
    ("Czech", "cs"),
    ("Filipino", "fil"),
    ("Persian", "fa"),
    ("Greek", "el"),
    ("Hungarian", "hu"),
    ("Macedonian", "mk"),
    ("Romanian", "ro"),
];

/// ISO 코드 → 모델이 아는 언어명(언어 강제용).
pub(crate) fn lang_name(code: &str) -> Option<&'static str> {
    LANGS.iter().find(|(_, c)| *c == code).map(|(n, _)| *n)
}

/// 모델이 낸 언어명 → ISO 코드. `Chinese,English` 처럼 여러 개면 첫 번째. 모르면 `en`.
pub(crate) fn lang_code(name: &str) -> &'static str {
    let first = name
        .split(|c: char| c == ',' || c.is_whitespace())
        .find(|s| !s.is_empty())
        .unwrap_or("");
    LANGS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(first))
        .map(|(_, c)| *c)
        .unwrap_or("en")
}

/// ChatML(GGUF 내장 템플릿과 같다). system 은 비우고 user 에 오디오 마커만 넣는다.
/// 언어를 강제하면 어시스턴트 턴을 `language X<asr_text>` 로 미리 채워 텍스트만 나오게 한다.
fn prompt(forced_name: Option<&str>) -> String {
    let mut p = format!(
        "<|im_start|>system\n<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
        mtmd_default_marker()
    );
    if let Some(n) = forced_name {
        p.push_str(&format!("language {n}{ASR_TAG}"));
    }
    p
}

/// `language X<asr_text>텍스트` → `(코드, 텍스트)`. 무음(`language None`)이나 빈 텍스트는 None.
/// 언어를 강제했으면 출력 전체가 텍스트다.
pub(crate) fn parse_output(raw: &str, forced: Option<&str>) -> Option<(String, String)> {
    let clean = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(code) = forced {
        return Some((code.to_string(), clean(s)));
    }
    let Some((meta, text)) = s.split_once(ASR_TAG) else {
        return Some(("en".to_string(), clean(s)));
    };
    let text = clean(text);
    if text.is_empty() {
        return None;
    }
    let name = meta
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("language "))
        .unwrap_or("")
        .trim();
    if name.eq_ignore_ascii_case("none") {
        return None;
    }
    Some((lang_code(name).to_string(), text))
}

pub struct Qwen3AsrTranscriber {
    // 드롭 순서: ctx → mtmd → model. ctx 와 mtmd 가 model 을 빌린다(local.rs 와 같은 'static 트릭).
    ctx: LlamaContext<'static>,
    mtmd: MtmdContext,
    model: Box<LlamaModel>,
    pub gpu_active: bool,
}

// SAFETY: llama_context 는 &mut 로만 쓰므로 한 번에 한 스레드만 접근한다.
unsafe impl Send for Qwen3AsrTranscriber {}

impl Qwen3AsrTranscriber {
    /// GPU 로드가 실패하면 CPU 로 한 번 더 시도한다. 두 번째 값은 그 폴백 여부.
    pub fn load(
        model: &Path,
        mmproj: &Path,
        use_gpu: bool,
    ) -> Result<(Self, bool), TranscribeError> {
        let try_load = |layers: u32| {
            LlamaModel::load_from_file(
                backend(),
                model,
                &LlamaModelParams::default().with_n_gpu_layers(layers),
            )
        };
        let (model, fell_back) = match try_load(if use_gpu { 1000 } else { 0 }) {
            Ok(m) => (m, false),
            Err(e) if use_gpu => {
                let m =
                    try_load(0).map_err(|e2| TranscribeError::Load(format!("{e}; cpu: {e2}")))?;
                (m, true)
            }
            Err(e) => return Err(TranscribeError::Load(e.to_string())),
        };
        let gpu = use_gpu && !fell_back;
        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        let model = Box::new(model);
        let mmproj = mmproj
            .to_str()
            .ok_or_else(|| TranscribeError::Load("mmproj path is not utf-8".into()))?;
        let mtmd = MtmdContext::init_from_file(
            mmproj,
            &model,
            &MtmdContextParams {
                use_gpu: gpu,
                print_timings: false,
                n_threads: threads,
                ..MtmdContextParams::default()
            },
        )
        .map_err(|e| TranscribeError::Load(e.to_string()))?;
        if !mtmd.support_audio() {
            return Err(TranscribeError::Load("mmproj has no audio encoder".into()));
        }
        let params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(N_CTX))
            .with_n_threads(threads)
            .with_n_threads_batch(threads);
        // SAFETY: model 은 Box 안에 있고 ctx 보다 늦게 드롭된다.
        let model_ref: &'static LlamaModel = unsafe { &*(&*model as *const LlamaModel) };
        let ctx = model_ref
            .new_context(backend(), params)
            .map_err(|e| TranscribeError::Load(e.to_string()))?;
        Ok((
            Self {
                ctx,
                mtmd,
                model,
                gpu_active: gpu && cfg!(any(feature = "metal", feature = "cuda")),
            },
            fell_back,
        ))
    }
}

impl Transcriber for Qwen3AsrTranscriber {
    fn transcribe(
        &mut self,
        pcm16k: &[f32],
        lang: Option<&str>,
    ) -> Result<Vec<Segment>, TranscribeError> {
        let inf = |e: &dyn std::fmt::Display| TranscribeError::Inference(e.to_string());
        // 1초 미만이면 0으로 채운다(whisper 와 같은 규칙; 인코더 창보다 짧은 입력을 피한다).
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
        let forced = lang.filter(|c| lang_name(c).is_some());
        let bitmap = MtmdBitmap::from_audio_data(pcm).map_err(|e| inf(&e))?;
        let chunks = self
            .mtmd
            .tokenize(
                MtmdInputText {
                    text: prompt(forced.and_then(lang_name)),
                    add_special: true,
                    parse_special: true,
                },
                &[&bitmap],
            )
            .map_err(|e| inf(&e))?;
        self.ctx.clear_kv_cache();
        let n_past = chunks
            .eval_chunks(&self.mtmd, &self.ctx, 0, 0, N_BATCH, true)
            .map_err(|e| inf(&e))?;

        let mut sampler = LlamaSampler::greedy();
        // 토큰 하나가 UTF-8 문자 중간에서 끊길 수 있어 상태 있는 디코더로 이어 붙인다.
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut out = String::new();
        let mut batch = LlamaBatch::new(1, 1);
        // -1 = 직전 decode 의 마지막 logits(eval_chunks 가 logits_last 로 남긴다).
        let mut idx = -1;
        for pos in (n_past..).take(MAX_NEW) {
            let tok = sampler.sample(&self.ctx, idx);
            if self.model.is_eog_token(tok) {
                break;
            }
            out.push_str(
                &self
                    .model
                    .token_to_piece(tok, &mut decoder, false, None)
                    .unwrap_or_default(),
            );
            batch.clear();
            batch.add(tok, pos, &[0], true).map_err(|e| inf(&e))?;
            self.ctx.decode(&mut batch).map_err(|e| inf(&e))?;
            idx = batch.n_tokens() - 1;
        }
        // ponytail: 반복 출력 억제 없음. 실측되면 n-gram 반복을 잘라내는 후처리를 여기에 둔다.
        Ok(parse_output(&out, forced)
            .map(|(lang, text)| Segment {
                text,
                lang,
                t0_ms: 0,
                t1_ms: 0,
            })
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_tag_and_text() {
        assert_eq!(
            parse_output("language Korean<asr_text>안녕하세요 여러분", None),
            Some(("ko".into(), "안녕하세요 여러분".into()))
        );
        assert_eq!(
            parse_output("language English\n<asr_text>hello   world\n", None),
            Some(("en".into(), "hello world".into()))
        );
    }

    #[test]
    fn silence_is_none() {
        assert_eq!(parse_output("language None<asr_text>", None), None);
        assert_eq!(parse_output("language None<asr_text>hm", None), None);
        assert_eq!(parse_output("   ", None), None);
        assert_eq!(parse_output("language Korean<asr_text>   ", None), None);
    }

    #[test]
    fn missing_tag_is_plain_english_text() {
        assert_eq!(
            parse_output("just text", None),
            Some(("en".into(), "just text".into()))
        );
    }

    #[test]
    fn forced_language_treats_output_as_text() {
        assert_eq!(
            parse_output("こんにちは", Some("ja")),
            Some(("ja".into(), "こんにちは".into()))
        );
    }

    #[test]
    fn language_names_map_to_codes() {
        assert_eq!(lang_code("Korean"), "ko");
        assert_eq!(lang_code("japanese"), "ja");
        assert_eq!(lang_code("Chinese,English"), "zh");
        assert_eq!(lang_code("Klingon"), "en");
        assert_eq!(lang_name("ko"), Some("Korean"));
        assert_eq!(lang_name("xx"), None);
    }

    #[test]
    fn prompt_ends_with_assistant_turn_or_forced_prefix() {
        let p = prompt(None);
        assert!(p.contains(llama_cpp_2::mtmd::mtmd_default_marker()));
        assert!(p.ends_with("<|im_start|>assistant\n"));
        assert!(prompt(Some("Korean")).ends_with("language Korean<asr_text>"));
    }

    /// 16 kHz 모노 16비트 PCM WAV → f32 샘플. `data` 청크만 찾아 읽는다(테스트 전용, 의존성 없이).
    fn read_wav_16k_mono(path: &str) -> Vec<f32> {
        let b = std::fs::read(path).unwrap();
        let u32at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize;
        let mut i = 12; // RIFF 헤더(12바이트) 다음부터 청크가 이어진다.
        while i + 8 <= b.len() {
            let size = u32at(i + 4);
            if &b[i..i + 4] == b"data" {
                let end = (i + 8 + size).min(b.len());
                return b[i + 8..end]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| i16::from_le_bytes(*c) as f32 / 32768.0)
                    .collect();
            }
            i += 8 + size + (size & 1); // 청크는 짝수 바이트 경계에 맞춰진다.
        }
        panic!("no data chunk in {path}");
    }

    #[test]
    #[ignore = "needs BABELAY_TEST_ASR_GGUF=<Qwen3-ASR-*.gguf> and BABELAY_TEST_ASR_MMPROJ=<mmproj-*.gguf> (+ optional BABELAY_TEST_ASR_WAV=<16k mono s16 wav>)"]
    fn loads_and_transcribes_silence_without_panicking() {
        use crate::transcribe::Transcriber;
        let model = std::env::var("BABELAY_TEST_ASR_GGUF").unwrap();
        let mmproj = std::env::var("BABELAY_TEST_ASR_MMPROJ").unwrap();
        let (mut t, fell_back) = Qwen3AsrTranscriber::load(
            std::path::Path::new(&model),
            std::path::Path::new(&mmproj),
            true,
        )
        .unwrap();
        eprintln!("gpu_active={} fell_back={}", t.gpu_active, fell_back);
        let started = std::time::Instant::now();
        let segs = t.transcribe(&vec![0.0f32; 16_000 * 3], None).unwrap();
        eprintln!("{segs:?} ({} ms)", started.elapsed().as_millis());
        assert!(segs.len() <= 1);
        // 짧은 입력도 패딩되어 패닉/에러 없이 지나가야 한다. 언어 강제 경로도 한 번.
        assert!(t.transcribe(&[0.0f32; 100], Some("ko")).is_ok());
        // 실제 영어 음성이 있으면 프롬프트/`<asr_text>` 규약이 GGUF 와 맞는지까지 확인한다.
        if let Ok(wav) = std::env::var("BABELAY_TEST_ASR_WAV") {
            let pcm = read_wav_16k_mono(&wav);
            let started = std::time::Instant::now();
            let segs = t.transcribe(&pcm, None).unwrap();
            eprintln!(
                "speech: {segs:?} ({} samples, {} ms)",
                pcm.len(),
                started.elapsed().as_millis()
            );
            let seg = segs.first().expect("speech must produce a segment");
            assert!(!seg.text.trim().is_empty());
            assert_eq!(seg.lang, "en");
        }
    }
}
