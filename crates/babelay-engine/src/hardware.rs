//! 하드웨어 감지와 사양 기반 balanced 추천.
use sysinfo::System;

#[derive(Clone, Debug, serde::Serialize)]
pub struct HwInfo {
    pub chip: String,
    pub mem_gb: u32,
    pub gpu: Option<String>,
    pub gpu_mem_gb: Option<u32>,
}

pub struct Balanced {
    pub asr: &'static str,
    pub llm: &'static str,
}

/// `System::new_all()`은 수십 ms 걸린다. 호출부에서 캐시할 것.
pub fn detect() -> HwInfo {
    let sys = System::new_all();
    let chip = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let mem_gb = (sys.total_memory() / (1 << 30)) as u32;
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
    let name = dev.name().ok();
    let vram = dev.memory_info().ok().map(|m| (m.total / (1 << 30)) as u32);
    (name, vram)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn gpu_info() -> (Option<String>, Option<u32>) {
    (None, None)
}

// ponytail: 고정 표. 실측 후 조정.
pub fn balanced(hw: &HwInfo) -> Balanced {
    let mem = hw.gpu_mem_gb.unwrap_or(hw.mem_gb);
    match (hw.gpu.is_some(), mem) {
        (true, m) if m >= 16 => Balanced {
            asr: "large-v3-turbo",
            llm: "qwen3.5-4b",
        },
        (true, m) if m >= 8 => Balanced {
            asr: "small",
            llm: "qwen3.5-2b",
        },
        _ => Balanced {
            asr: "base",
            llm: "gemma3-1b",
        },
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
    fn balanced_ids_exist() {
        let b = balanced(&hw(true, 16, None));
        assert!(crate::models::find(b.asr).is_some() && crate::models::find(b.llm).is_some());
    }
}
