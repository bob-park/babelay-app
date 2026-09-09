//! 모델 레지스트리. size_bytes/sha256 은 HuggingFace API 실측값(2026-09-09).
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Asr,
    Llm,
}

/// 내려받는 파일 하나. 본체는 `ModelInfo` 의 url/filename/size_bytes/sha256 이고,
/// Qwen3-ASR 처럼 오디오 인코더가 따로 있는 모델은 `mmproj` 로 하나 더 갖는다.
#[derive(Serialize, Clone, Copy, Debug)]
pub struct ModelFile {
    pub url: &'static str,
    pub filename: &'static str,
    pub size_bytes: u64,
    pub sha256: Option<&'static str>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ModelInfo {
    pub id: &'static str,
    pub kind: Kind,
    pub name: &'static str,
    pub desc_key: &'static str,
    /// 본체 파일 크기.
    pub size_bytes: u64,
    /// 본체 + mmproj. UI 가 보여주는 용량.
    pub total_bytes: u64,
    /// 1(느림)..5(빠름).
    pub speed: u8,
    /// 1(낮음)..5(높음). 정확도 등급.
    pub quality: u8,
    pub url: &'static str,
    pub filename: &'static str,
    pub sha256: Option<&'static str>,
    pub mmproj: Option<ModelFile>,
}

impl ModelInfo {
    pub fn main_file(&self) -> ModelFile {
        ModelFile {
            url: self.url,
            filename: self.filename,
            size_bytes: self.size_bytes,
            sha256: self.sha256,
        }
    }

    /// 본체 먼저, 그다음 mmproj.
    pub fn files(&self) -> impl Iterator<Item = ModelFile> + '_ {
        std::iter::once(self.main_file()).chain(self.mmproj)
    }
}

const fn extra_bytes(f: Option<ModelFile>) -> u64 {
    match f {
        Some(f) => f.size_bytes,
        None => 0,
    }
}

macro_rules! m {
    ($id:literal, $kind:ident, $name:literal, $desc:literal, $size:literal, $speed:literal, $quality:literal, $url:expr, $file:literal, $sha:expr) => {
        m!($id, $kind, $name, $desc, $size, $speed, $quality, $url, $file, $sha, None)
    };
    ($id:literal, $kind:ident, $name:literal, $desc:literal, $size:literal, $speed:literal, $quality:literal, $url:expr, $file:literal, $sha:expr, $mmproj:expr) => {
        ModelInfo {
            id: $id,
            kind: Kind::$kind,
            name: $name,
            desc_key: $desc,
            size_bytes: $size,
            total_bytes: $size + extra_bytes($mmproj),
            speed: $speed,
            quality: $quality,
            url: $url,
            filename: $file,
            sha256: $sha,
            mmproj: $mmproj,
        }
    };
}

pub const REGISTRY: &[ModelInfo] = &[
    m!(
        "tiny",
        Asr,
        "Whisper Tiny",
        "models.desc.tiny",
        77_691_713,
        5,
        1,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        "ggml-tiny.bin",
        Some("be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21")
    ),
    m!(
        "base",
        Asr,
        "Whisper Base",
        "models.desc.base",
        147_951_465,
        4,
        2,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        "ggml-base.bin",
        Some("60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe")
    ),
    m!(
        "small",
        Asr,
        "Whisper Small",
        "models.desc.small",
        487_601_967,
        3,
        3,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        "ggml-small.bin",
        Some("1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b")
    ),
    m!(
        "medium",
        Asr,
        "Whisper Medium",
        "models.desc.medium",
        1_533_763_059,
        2,
        4,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        "ggml-medium.bin",
        Some("6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208")
    ),
    m!(
        "large-v3-turbo",
        Asr,
        "Whisper Large v3 Turbo",
        "models.desc.large_v3_turbo",
        1_624_555_275,
        2,
        4,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        "ggml-large-v3-turbo.bin",
        Some("1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69")
    ),
    m!(
        "large-v3",
        Asr,
        "Whisper Large v3",
        "models.desc.large_v3",
        3_095_033_483,
        1,
        5,
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        "ggml-large-v3.bin",
        Some("64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2")
    ),
    m!(
        "qwen3-asr-0.6b",
        Asr,
        "Qwen3-ASR 0.6B",
        "models.desc.qwen3_asr_0_6b",
        804_749_248,
        3,
        4,
        "https://huggingface.co/ggml-org/Qwen3-ASR-0.6B-GGUF/resolve/main/Qwen3-ASR-0.6B-Q8_0.gguf",
        "Qwen3-ASR-0.6B-Q8_0.gguf",
        Some("bca259818b50ca7c4c05e9bdb35a5dc04fa039653a6d6f3f0f331f96f6aa1971"),
        Some(ModelFile {
            url: "https://huggingface.co/ggml-org/Qwen3-ASR-0.6B-GGUF/resolve/main/mmproj-Qwen3-ASR-0.6B-Q8_0.gguf",
            filename: "mmproj-Qwen3-ASR-0.6B-Q8_0.gguf",
            size_bytes: 214_392_480,
            sha256: Some("41a342b5e4c514e968cb756de6cd1b7be39eff43c44c57a2ef5fc6522e36603d"),
        })
    ),
    m!(
        "qwen3-asr-1.7b",
        Asr,
        "Qwen3-ASR 1.7B",
        "models.desc.qwen3_asr_1_7b",
        2_165_034_944,
        2,
        5,
        "https://huggingface.co/ggml-org/Qwen3-ASR-1.7B-GGUF/resolve/main/Qwen3-ASR-1.7B-Q8_0.gguf",
        "Qwen3-ASR-1.7B-Q8_0.gguf",
        Some("58e22d0532d4eacaf034cfac17a6fed159f37c41390c710186783be439d1fc57"),
        Some(ModelFile {
            url: "https://huggingface.co/ggml-org/Qwen3-ASR-1.7B-GGUF/resolve/main/mmproj-Qwen3-ASR-1.7B-Q8_0.gguf",
            filename: "mmproj-Qwen3-ASR-1.7B-Q8_0.gguf",
            size_bytes: 355_709_344,
            sha256: Some("46c1d533af3f354ceb37ce855dbceff7da7fa7cf1e6a523df3b13440bd164c0d"),
        })
    ),
    m!(
        "gemma3-1b",
        Llm,
        "Gemma 3 1B",
        "models.desc.gemma3_1b",
        806_058_272,
        5,
        1,
        "https://huggingface.co/unsloth/gemma-3-1b-it-GGUF/resolve/main/gemma-3-1b-it-Q4_K_M.gguf",
        "gemma-3-1b-it-Q4_K_M.gguf",
        Some("8270790f3ab69fdfe860b7b64008d9a19986d8df7e407bb018184caa08798ebd")
    ),
    m!(
        "gemma3-4b",
        Llm,
        "Gemma 3 4B",
        "models.desc.gemma3_4b",
        2_489_894_016,
        3,
        3,
        "https://huggingface.co/unsloth/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf",
        "gemma-3-4b-it-Q4_K_M.gguf",
        Some("04a43a22e8d2003deda5acc262f68ec1005fa76c735a9962a8c77042a74a7d19")
    ),
    m!(
        "qwen3-1.7b",
        Llm,
        "Qwen 3 1.7B",
        "models.desc.qwen3_1_7b",
        1_107_409_472,
        4,
        2,
        "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q4_K_M.gguf",
        "Qwen3-1.7B-Q4_K_M.gguf",
        Some("b139949c5bd74937ad8ed8c8cf3d9ffb1e99c866c823204dc42c0d91fa181897")
    ),
    m!(
        "qwen3-4b",
        Llm,
        "Qwen 3 4B",
        "models.desc.qwen3_4b",
        2_497_281_312,
        2,
        4,
        "https://huggingface.co/unsloth/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q4_K_M.gguf",
        "Qwen3-4B-Q4_K_M.gguf",
        Some("f6f851777709861056efcdad3af01da38b31223a3ba26e61a4f8bf3a2195813a")
    ),
    m!(
        "qwen3.5-2b",
        Llm,
        "Qwen 3.5 2B",
        "models.desc.qwen3_5_2b",
        1_280_835_840,
        4,
        3,
        "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
        "Qwen3.5-2B-Q4_K_M.gguf",
        Some("aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223")
    ),
    m!(
        "qwen3.5-4b",
        Llm,
        "Qwen 3.5 4B",
        "models.desc.qwen3_5_4b",
        2_740_937_888,
        3,
        4,
        "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf",
        "Qwen3.5-4B-Q4_K_M.gguf",
        Some("00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4")
    ),
    m!(
        "hy-mt1.5-1.8b",
        Llm,
        "HY-MT1.5 1.8B",
        "models.desc.hy_mt1_5_1_8b",
        1_133_080_512,
        4,
        3,
        "https://huggingface.co/tencent/HY-MT1.5-1.8B-GGUF/resolve/main/HY-MT1.5-1.8B-Q4_K_M.gguf",
        "HY-MT1.5-1.8B-Q4_K_M.gguf",
        Some("4383ac0c3c8e476de98ff979c2a3f069f8c4fb385e7860cf2d28da896cc477c7")
    ),
    m!(
        "hy-mt2-1.8b",
        Llm,
        "Hy-MT2 1.8B",
        "models.desc.hy_mt2_1_8b",
        1_133_080_448,
        4,
        4,
        "https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/resolve/main/Hy-MT2-1.8B-Q4_K_M.gguf",
        "Hy-MT2-1.8B-Q4_K_M.gguf",
        Some("dc5f44fcf1fa496ee7ad725982c0c8c553a4de00259b53af84c4b89fb0c06699")
    ),
    m!(
        "hy-mt1.5-7b",
        Llm,
        "HY-MT1.5 7B",
        "models.desc.hy_mt1_5_7b",
        4_624_649_312,
        1,
        4,
        "https://huggingface.co/tencent/HY-MT1.5-7B-GGUF/resolve/main/HY-MT1.5-7B-Q4_K_M.gguf",
        "HY-MT1.5-7B-Q4_K_M.gguf",
        Some("fc87637e4dd29547811a28170770c2ac17725fb7690b7c4aafa4f463c3e77568")
    ),
    m!(
        "hy-mt2-7b",
        Llm,
        "Hy-MT2 7B",
        "models.desc.hy_mt2_7b",
        4_624_648_896,
        1,
        5,
        "https://huggingface.co/tencent/Hy-MT2-7B-GGUF/resolve/main/Hy-MT2-7B-Q4_K_M.gguf",
        "Hy-MT2-7B-Q4_K_M.gguf",
        Some("9f96256500f3fc1ab4d64336b58f52a949a95ad7516b0c229476eef782f9f77b")
    ),
];

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    REGISTRY.iter().find(|m| m.id == id)
}

fn kind_dir(m: &ModelInfo) -> &'static str {
    match m.kind {
        Kind::Asr => "asr",
        Kind::Llm => "llm",
    }
}

/// 본체 파일 경로.
pub fn model_path(models_dir: &Path, m: &ModelInfo) -> PathBuf {
    models_dir.join(kind_dir(m)).join(m.filename)
}

/// 모델에 속한 파일 하나의 경로. mmproj 도 본체와 같은 디렉터리에 둔다.
pub fn file_path(models_dir: &Path, m: &ModelInfo, f: &ModelFile) -> PathBuf {
    models_dir.join(kind_dir(m)).join(f.filename)
}

pub fn installed(models_dir: &Path, m: &ModelInfo) -> bool {
    m.files().all(|f| {
        std::fs::metadata(file_path(models_dir, m, &f))
            .map(|md| md.is_file() && md.len() == f.size_bytes)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_and_urls_are_https() {
        let mut seen = HashSet::new();
        for m in REGISTRY {
            assert!(seen.insert(m.id), "duplicate id {}", m.id);
            assert!(m.url.starts_with("https://"), "{}", m.id);
            assert!(m.size_bytes > 0, "{}", m.id);
            assert!((1..=5).contains(&m.speed), "{}", m.id);
            assert!(m.desc_key.starts_with("models.desc."), "{}", m.id);
        }
    }

    #[test]
    fn installed_requires_exact_size() {
        let dir = tempfile::tempdir().unwrap();
        let m = find("tiny").unwrap();
        assert!(!installed(dir.path(), m));
        let p = model_path(dir.path(), m);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, vec![0u8; 10]).unwrap();
        assert!(!installed(dir.path(), m), "wrong size must not count");
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(m.size_bytes).unwrap();
        assert!(installed(dir.path(), m));
    }

    #[test]
    fn quality_in_range_and_total_includes_mmproj() {
        for m in REGISTRY {
            assert!((1..=5).contains(&m.quality), "{}", m.id);
            let extra = m.mmproj.map(|f| f.size_bytes).unwrap_or(0);
            assert_eq!(m.total_bytes, m.size_bytes + extra, "{}", m.id);
            if let Some(f) = m.mmproj {
                assert!(f.url.starts_with("https://"), "{}", m.id);
                assert!(
                    f.size_bytes > 0 && f.filename.starts_with("mmproj-"),
                    "{}",
                    m.id
                );
                assert_eq!(m.kind, Kind::Asr, "{}", m.id);
            }
        }
    }

    #[test]
    fn new_models_are_registered_with_expected_kind() {
        for (id, kind) in [
            ("qwen3-asr-1.7b", Kind::Asr),
            ("qwen3-asr-0.6b", Kind::Asr),
            ("hy-mt2-1.8b", Kind::Llm),
            ("hy-mt2-7b", Kind::Llm),
            ("hy-mt1.5-1.8b", Kind::Llm),
            ("hy-mt1.5-7b", Kind::Llm),
        ] {
            let m = find(id).unwrap_or_else(|| panic!("{id} missing"));
            assert_eq!(m.kind, kind, "{id}");
        }
        assert!(find("qwen3-asr-0.6b").unwrap().mmproj.is_some());
        assert!(find("hy-mt2-1.8b").unwrap().mmproj.is_none());
        assert_eq!(find("qwen3-asr-0.6b").unwrap().files().count(), 2);
        assert_eq!(find("tiny").unwrap().files().count(), 1);
    }

    #[test]
    fn installed_requires_every_file() {
        let dir = tempfile::tempdir().unwrap();
        let m = find("qwen3-asr-0.6b").unwrap();
        let main = model_path(dir.path(), m);
        std::fs::create_dir_all(main.parent().unwrap()).unwrap();
        std::fs::File::create(&main)
            .unwrap()
            .set_len(m.size_bytes)
            .unwrap();
        assert!(!installed(dir.path(), m), "mmproj missing must not count");
        let mm = m.mmproj.unwrap();
        let mm_path = file_path(dir.path(), m, &mm);
        std::fs::File::create(&mm_path)
            .unwrap()
            .set_len(mm.size_bytes)
            .unwrap();
        assert!(installed(dir.path(), m));
        assert_eq!(mm_path.parent(), main.parent());
    }
}
