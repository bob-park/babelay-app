//! 모델 파일 다운로드: 이어받기·검증·취소.

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub received: u64,
    pub total: u64,
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("http: {0}")]
    Http(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("cancelled")]
    Cancelled,
    #[error("verification failed: {0}")]
    Mismatch(String),
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// `dest.part`에 이어받아 검증 후 `dest`로 옮긴다.
/// 매 8KB 청크마다 `cancel`을 확인하며, 취소 시 `.part`는 남긴다.
pub fn download(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
    expected_size: u64,
    sha256: Option<&str>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<(), DownloadError> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let part = part_path(dest);
    let mut have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let mut req = client.get(url);
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let mut resp = req.send().map_err(|e| DownloadError::Http(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(DownloadError::Http(format!("{status} for {url}")));
    }
    let resuming = status.as_u16() == 206;
    if !resuming {
        have = 0; // 서버가 Range 를 무시했으면 처음부터
    }
    let remaining = resp
        .content_length()
        .unwrap_or(expected_size.saturating_sub(have));
    let total = have + remaining;

    let mut file = if resuming {
        OpenOptions::new().append(true).open(&part)?
    } else {
        File::create(&part)?
    };

    let mut buf = [0u8; 8192];
    let mut received = have;
    on_progress(Progress { received, total });
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let n = resp
            .read(&mut buf)
            .map_err(|e| DownloadError::Http(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        received += n as u64;
        on_progress(Progress { received, total });
    }
    file.flush()?;
    drop(file);

    let ok = match sha256 {
        Some(expected) => {
            let mut hasher = Sha256::new();
            let mut f = File::open(&part)?;
            let mut chunk = [0u8; 65536];
            loop {
                let n = f.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                hasher.update(&chunk[..n]);
            }
            format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected)
        }
        None => fs::metadata(&part)?.len() == expected_size,
    };
    if !ok {
        let _ = fs::remove_file(&part);
        return Err(DownloadError::Mismatch(format!("{}", dest.display())));
    }
    fs::rename(&part, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use std::sync::atomic::AtomicBool;

    const BODY: &[u8] = b"0123456789abcdef";

    fn client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::new()
    }

    #[test]
    fn downloads_whole_file_and_reports_progress() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).header("content-length", "16").body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        let mut seen = vec![];
        download(
            &client(),
            &server.url("/m.bin"),
            &dest,
            16,
            None,
            &AtomicBool::new(false),
            &mut |p| seen.push(p.received),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), BODY);
        assert_eq!(*seen.last().unwrap(), 16);
        assert!(!dest.with_extension("bin.part").exists());
    }

    #[test]
    fn resumes_from_existing_part_with_range_header() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin").header("range", "bytes=6-");
            t.status(206)
                .header("content-length", "10")
                .body(&BODY[6..]);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        std::fs::write(dest.with_extension("bin.part"), &BODY[..6]).unwrap();
        download(
            &client(),
            &server.url("/m.bin"),
            &dest,
            16,
            None,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), BODY);
    }

    #[test]
    fn size_mismatch_fails_and_removes_part() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        let err = download(
            &client(),
            &server.url("/m.bin"),
            &dest,
            99,
            None,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, DownloadError::Mismatch(_)));
        assert!(!dest.exists() && !dest.with_extension("bin.part").exists());
    }

    #[test]
    fn sha256_mismatch_fails() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        let err = download(
            &client(),
            &server.url("/m.bin"),
            &dest,
            16,
            Some(&"0".repeat(64)),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, DownloadError::Mismatch(_)));
    }

    #[test]
    fn cancel_keeps_part_file() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).header("content-length", "16").body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        let cancel = AtomicBool::new(true);
        let err = download(
            &client(),
            &server.url("/m.bin"),
            &dest,
            16,
            None,
            &cancel,
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, DownloadError::Cancelled));
        assert!(dest.with_extension("bin.part").exists());
        assert!(!dest.exists());
    }

    #[test]
    fn http_error_status_is_reported() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(404);
        });
        let dir = tempfile::tempdir().unwrap();
        let err = download(
            &client(),
            &server.url("/m.bin"),
            &dir.path().join("m.bin"),
            16,
            None,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, DownloadError::Http(_)));
    }
}
