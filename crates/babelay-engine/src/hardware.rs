//! 하드웨어 감지와 사양 기반 balanced 추천.
use crate::models::ModelInfo;
use sysinfo::System;

#[derive(Clone, Debug, serde::Serialize)]
pub struct HwInfo {
    pub chip: String,
    pub mem_gb: u32,
    pub gpu: Option<String>,
    pub gpu_mem_gb: Option<u32>,
}

/// 실제 하드웨어는 16 GiB를 15.9 GiB로 보고한다. 가장 가까운 GiB로 반올림한다.
fn to_gb(bytes: u64) -> u32 {
    ((bytes + (1u64 << 29)) >> 30) as u32
}

/// `System::new_all()`은 수십 ms 걸린다. 호출부에서 캐시할 것.
pub fn detect() -> HwInfo {
    let sys = System::new_all();
    let chip = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let mem_gb = to_gb(sys.total_memory());
    let (gpu, gpu_mem_gb) = gpu_info();
    HwInfo {
        chip,
        mem_gb,
        gpu,
        gpu_mem_gb,
    }
}

#[cfg(target_os = "macos")]
fn gpu_info() -> (Option<String>, Option<u32>) {
    // 통합 메모리라 VRAM은 따로 없다 → mem_gb로 판정한다.
    if cfg!(target_arch = "aarch64") {
        (Some("Apple Silicon (Metal)".into()), None)
    } else {
        (None, None)
    }
}

#[cfg(target_os = "windows")]
fn gpu_info() -> (Option<String>, Option<u32>) {
    let Ok(nvml) = nvml_wrapper::Nvml::init() else {
        return (None, None);
    };
    let Ok(dev) = nvml.device_by_index(0) else {
        return (None, None);
    };
    // ponytail: NVIDIA만 본다. AMD/Intel GPU는 CPU 행으로 떨어진다.
    let name = dev.name().ok();
    let vram = dev.memory_info().ok().map(|m| to_gb(m.total));
    (name, vram)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn gpu_info() -> (Option<String>, Option<u32>) {
    (None, None)
}

pub struct Balanced {
    pub asr: &'static str,
    pub llm: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    pub id: &'static str,
    pub asr: &'static str,
    pub llm: &'static str,
}

const fn preset(id: &'static str, asr: &'static str, llm: &'static str) -> Preset {
    Preset { id, asr, llm }
}

/// 사양 등급별 프리셋. 순서는 fast, balanced, quality.
/// ponytail: 고정 표. Qwen3-ASR/HY-MT 실측 후 균형 등급에 편입할지 정한다.
pub fn presets(hw: &HwInfo) -> [Preset; 3] {
    let mem = hw.gpu_mem_gb.unwrap_or(hw.mem_gb);
    match (hw.gpu.is_some(), mem) {
        (true, m) if m >= 16 => [
            preset("fast", "small", "qwen3.5-2b"),
            preset("balanced", "large-v3-turbo", "qwen3.5-4b"),
            preset("quality", "qwen3-asr-1.7b", "hy-mt2-7b"),
        ],
        (true, m) if m >= 8 => [
            preset("fast", "base", "gemma3-1b"),
            preset("balanced", "small", "qwen3.5-2b"),
            preset("quality", "qwen3-asr-0.6b", "hy-mt2-1.8b"),
        ],
        _ => [
            preset("fast", "tiny", "gemma3-1b"),
            preset("balanced", "base", "gemma3-1b"),
            preset("quality", "small", "qwen3.5-2b"),
        ],
    }
}

/// 추천(균형) 프리셋. `presets()[1]` 과 같다.
pub fn balanced(hw: &HwInfo) -> Balanced {
    let p = presets(hw)[1];
    Balanced {
        asr: p.asr,
        llm: p.llm,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    Good,
    Heavy,
}

/// 이 기기에서 버거울지 경험칙으로 판정한다.
/// 예산: NVIDIA 는 VRAM/2, Apple Silicon(통합 메모리)은 RAM/3, CPU 전용은 RAM/4.
/// CPU 전용에서 Qwen3-ASR(mmproj 있는 모델)는 실시간을 못 따라가므로 항상 Heavy.
/// ponytail: 단순 경험칙. 로드 실패·지연 실측 후 비율을 조정한다.
pub fn fit(hw: &HwInfo, m: &ModelInfo) -> Fit {
    let (budget_gb, cpu_only) = match (hw.gpu.is_some(), hw.gpu_mem_gb) {
        (true, Some(vram)) => (vram / 2, false),
        (true, None) => (hw.mem_gb / 3, false),
        (false, _) => (hw.mem_gb / 4, true),
    };
    if cpu_only && m.mmproj.is_some() {
        return Fit::Heavy;
    }
    if m.total_bytes > (budget_gb as u64) << 30 {
        Fit::Heavy
    } else {
        Fit::Good
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hw(gpu: bool, mem: u32, vram: Option<u32>) -> HwInfo {
        HwInfo {
            chip: "x".into(),
            mem_gb: mem,
            gpu: gpu.then(|| "g".to_string()),
            gpu_mem_gb: vram,
        }
    }
    #[test]
    fn gpu_16gb_is_turbo_and_4b() {
        let b = balanced(&hw(true, 16, None));
        assert_eq!((b.asr, b.llm), ("large-v3-turbo", "qwen3.5-4b"));
    }
    #[test]
    fn gpu_8gb_is_small_and_2b() {
        let b = balanced(&hw(true, 8, None));
        assert_eq!((b.asr, b.llm), ("small", "qwen3.5-2b"));
    }
    #[test]
    fn cpu_only_is_base_and_gemma() {
        let b = balanced(&hw(false, 64, None));
        assert_eq!((b.asr, b.llm), ("base", "gemma3-1b"));
    }
    #[test]
    fn nvidia_uses_vram_not_ram() {
        let b = balanced(&hw(true, 64, Some(6)));
        assert_eq!(b.asr, "base");
    }
    #[test]
    fn balanced_ids_exist_with_matching_kind() {
        use crate::models::{find, Kind};
        for h in [hw(true, 16, None), hw(true, 8, None), hw(false, 4, None)] {
            let b = balanced(&h);
            assert_eq!(find(b.asr).unwrap().kind, Kind::Asr, "{}", b.asr);
            assert_eq!(find(b.llm).unwrap().kind, Kind::Llm, "{}", b.llm);
        }
    }

    #[test]
    fn presets_are_fast_balanced_quality_and_balanced_matches() {
        for h in [hw(true, 16, None), hw(true, 8, None), hw(false, 8, None)] {
            let p = presets(&h);
            assert_eq!([p[0].id, p[1].id, p[2].id], ["fast", "balanced", "quality"]);
            let b = balanced(&h);
            assert_eq!((b.asr, b.llm), (p[1].asr, p[1].llm));
            for x in &p {
                assert_eq!(
                    crate::models::find(x.asr).unwrap().kind,
                    crate::models::Kind::Asr,
                    "{}",
                    x.asr
                );
                assert_eq!(
                    crate::models::find(x.llm).unwrap().kind,
                    crate::models::Kind::Llm,
                    "{}",
                    x.llm
                );
            }
        }
    }

    #[test]
    fn quality_preset_uses_qwen_asr_and_hy_mt_on_gpu() {
        let p = presets(&hw(true, 16, None));
        assert_eq!((p[2].asr, p[2].llm), ("qwen3-asr-1.7b", "hy-mt2-7b"));
        let p = presets(&hw(true, 8, None));
        assert_eq!((p[2].asr, p[2].llm), ("qwen3-asr-0.6b", "hy-mt2-1.8b"));
        let p = presets(&hw(false, 32, None));
        assert_eq!((p[2].asr, p[2].llm), ("small", "qwen3.5-2b"));
    }

    #[test]
    fn fit_uses_memory_budget_and_cpu_rule() {
        use crate::models::find;
        let big = find("hy-mt2-7b").unwrap(); // 4.3 GiB
        let small = find("tiny").unwrap();
        let qwen_asr = find("qwen3-asr-0.6b").unwrap();
        // Apple 16 GB: 예산 5 GiB → 7B 도 Good
        assert_eq!(fit(&hw(true, 16, None), big), Fit::Good);
        // Apple 8 GB: 예산 2 GiB → Heavy
        assert_eq!(fit(&hw(true, 8, None), big), Fit::Heavy);
        // NVIDIA 8 GB VRAM: 예산 4 GiB → Heavy
        assert_eq!(fit(&hw(true, 64, Some(8)), big), Fit::Heavy);
        // CPU 전용은 Qwen3-ASR 가 항상 Heavy, 작은 모델은 Good
        assert_eq!(fit(&hw(false, 64, None), qwen_asr), Fit::Heavy);
        assert_eq!(fit(&hw(false, 4, None), small), Fit::Good);
    }

    #[test]
    fn to_gb_rounds_to_nearest() {
        assert_eq!(to_gb(17_179_869_184), 16);
        assert_eq!(to_gb(17_070_000_000), 16);
        assert_eq!(to_gb(8_514_043_904), 8);
        assert_eq!(to_gb(4_294_967_296), 4);
    }
}
