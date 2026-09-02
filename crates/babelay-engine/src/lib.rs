//! Babelay 엔진. 모델 레지스트리·다운로드. 오디오 캡처·전사·번역은 2단계.
pub mod download;
pub mod models;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_not_empty() {
        assert!(!super::version().is_empty());
    }
}
