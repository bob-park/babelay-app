//! 모델 레지스트리. size_bytes/sha256 은 HuggingFace HEAD 실측값(2026-09-03).
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Asr,
    Llm,
}

#[derive(Serialize, Clone, Debug)]
pub struct ModelInfo {
    pub id: &'static str,
    pub kind: Kind,
    pub name: &'static str,
    pub desc_key: &'static str,
    pub size_bytes: u64,
    pub speed: u8,
    pub url: &'static str,
    pub filename: &'static str,
    pub sha256: Option<&'static str>,
}

macro_rules! m {
    ($id:literal, $kind:ident, $name:literal, $desc:literal, $size:literal, $speed:literal, $url:expr, $file:literal, $sha:expr) => {
        ModelInfo {
            id: $id,
            kind: Kind::$kind,
            name: $name,
            desc_key: $desc,
            size_bytes: $size,
            speed: $speed,
            url: $url,
            filename: $file,
            sha256: $sha,
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
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        "ggml-large-v3.bin",
        Some("64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2")
    ),
    m!(
        "gemma3-1b",
        Llm,
        "Gemma 3 1B",
        "models.desc.gemma3_1b",
        806_058_272,
        5,
        "https://huggingface.co/unsloth/gemma-3-1b-it-GGUF/resolve/main/gemma-3-1b-it-Q4_K_M.gguf",
        "gemma-3-1b-it-Q4_K_M.gguf",
        Some("8270790f3ab69fdfe860b7b64008d9a19986d8df7e407bb018184caa08798ebd")
    ),
    m!(
        "qwen3.5-2b",
        Llm,
        "Qwen 3.5 2B",
        "models.desc.qwen3_5_2b",
        1_280_835_840,
        4,
        "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
        "Qwen3.5-2B-Q4_K_M.gguf",
        Some("aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223")
    ),
    m!(
        "gemma3-4b",
        Llm,
        "Gemma 3 4B",
        "models.desc.gemma3_4b",
        2_489_894_016,
        3,
        "https://huggingface.co/unsloth/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf",
        "gemma-3-4b-it-Q4_K_M.gguf",
        Some("04a43a22e8d2003deda5acc262f68ec1005fa76c735a9962a8c77042a74a7d19")
    ),
    m!(
        "qwen3.5-4b",
        Llm,
        "Qwen 3.5 4B",
        "models.desc.qwen3_5_4b",
        2_740_937_888,
        3,
        "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf",
        "Qwen3.5-4B-Q4_K_M.gguf",
        Some("00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4")
    ),
];

pub struct Balanced {
    pub asr: &'static str,
    pub llm: &'static str,
}

// ponytail: 고정값. 2단계에서 sysinfo/nvml 기반 판정으로 교체한다.
pub const BALANCED: Balanced = Balanced {
    asr: "small",
    llm: "qwen3.5-2b",
};

pub fn find(id: &str) -> Option<&'static ModelInfo> {
    REGISTRY.iter().find(|m| m.id == id)
}

pub fn model_path(models_dir: &Path, m: &ModelInfo) -> PathBuf {
    let sub = match m.kind {
        Kind::Asr => "asr",
        Kind::Llm => "llm",
    };
    models_dir.join(sub).join(m.filename)
}

pub fn installed(models_dir: &Path, m: &ModelInfo) -> bool {
    std::fs::metadata(model_path(models_dir, m))
        .map(|md| md.is_file() && md.len() == m.size_bytes)
        .unwrap_or(false)
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
    fn balanced_ids_exist_with_matching_kind() {
        assert_eq!(find(BALANCED.asr).unwrap().kind, Kind::Asr);
        assert_eq!(find(BALANCED.llm).unwrap().kind, Kind::Llm);
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
}
