//! API 키 저장. OS 자격 증명 저장소(macOS Keychain / Windows Credential Manager)만 쓴다 —
//! 설정 파일에는 절대 쓰지 않는다. service = 앱 번들 id, user = 프로바이더.
use keyring::{Entry, Error};

const SERVICE: &str = "org.bobpark.babelay";
const PROVIDERS: [&str; 5] = ["openai", "anthropic", "gemini", "deepl", "custom"];

fn entry(provider: &str) -> Result<Entry, String> {
    if !PROVIDERS.contains(&provider) {
        return Err("unknown_provider".into());
    }
    Entry::new(SERVICE, provider).map_err(|e| e.to_string())
}

/// 빈 키는 삭제로 취급한다.
pub fn set(provider: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return delete(provider);
    }
    entry(provider)?
        .set_password(key)
        .map_err(|e| e.to_string())
}

pub fn get(provider: &str) -> Result<Option<String>, String> {
    match entry(provider)?.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 없어도 성공.
pub fn delete(provider: &str) -> Result<(), String> {
    match entry(provider)?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn has(provider: &str) -> bool {
    matches!(get(provider), Ok(Some(_)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_provider_is_rejected() {
        assert_eq!(super::set("bogus", "k"), Err("unknown_provider".into()));
        assert!(!super::has("bogus"));
    }

    #[test]
    #[ignore = "touches the OS keychain (may prompt)"]
    fn roundtrip() {
        super::set("custom", "test-key").unwrap();
        assert_eq!(super::get("custom").unwrap().as_deref(), Some("test-key"));
        assert!(super::has("custom"));
        super::delete("custom").unwrap();
        assert_eq!(super::get("custom").unwrap(), None);
        super::delete("custom").unwrap(); // 두 번 지워도 성공
    }
}
