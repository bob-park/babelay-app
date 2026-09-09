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
    // 예상 크기 이상으로 자란 .part 는 이어받기가 416으로 막힌다. 버리고 처음부터.
    if have >= expected_size {
        let _ = fs::remove_file(&part);
        have = 0;
    }

    let get = |from: u64| {
        let mut req = client.get(url);
        if from > 0 {
            req = req.header("Range", format!("bytes={from}-"));
        }
        req.send().map_err(|e| DownloadError::Http(e.to_string()))
    };
    let mut resp = get(have)?;
    if resp.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // 서버가 이어받기를 거부하면 .part 를 버리고 한 번만 처음부터 다시.
        let _ = fs::remove_file(&part);
        have = 0;
        resp = get(0)?;
    }
    let status = resp.status();
    if !status.is_success() {
        return Err(DownloadError::Http(format!("{status} for {url}")));
    }
    let resuming = status.as_u16() == 206 && have > 0;
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

/// 모델에 속한 파일을 본체 → mmproj 순서로 받는다. 진행률의 `total` 은 모델 전체 크기이고
/// `received` 는 앞 파일까지 누적이다. 이미 정확한 크기로 있는 파일은 건너뛴다.
pub fn download_model(
    client: &reqwest::blocking::Client,
    models_dir: &Path,
    m: &crate::models::ModelInfo,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<(), DownloadError> {
    let total = m.total_bytes;
    let mut done = 0u64;
    for f in m.files() {
        let dest = crate::models::file_path(models_dir, m, &f);
        let present = fs::metadata(&dest)
            .map(|md| md.is_file() && md.len() == f.size_bytes)
            .unwrap_or(false);
        if !present {
            download(
                client,
                f.url,
                &dest,
                f.size_bytes,
                f.sha256,
                cancel,
                &mut |p| {
                    on_progress(Progress {
                        received: done + p.received.min(f.size_bytes),
                        total,
                    })
                },
            )?;
        }
        done += f.size_bytes;
        on_progress(Progress {
            received: done,
            total,
        });
    }
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
    fn unsolicited_206_without_part_restarts() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(206).header("content-length", "16").body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
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
    fn oversized_part_restarts_from_scratch() {
        let server = MockServer::start();
        let ranged = server.mock(|w, t| {
            w.method(GET).path("/m.bin").header_exists("range");
            t.status(416);
        });
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).header("content-length", "16").body(BODY);
        });
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        std::fs::write(dest.with_extension("bin.part"), vec![7u8; 20]).unwrap();
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
        assert!(!dest.with_extension("bin.part").exists());
        ranged.assert_hits(0);
    }

    #[test]
    fn range_not_satisfiable_restarts_without_range() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/m.bin").header_exists("range");
            t.status(416);
        });
        server.mock(|w, t| {
            w.method(GET).path("/m.bin");
            t.status(200).header("content-length", "16").body(BODY);
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
        assert!(!dest.with_extension("bin.part").exists());
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

    fn two_file_model(server: &MockServer) -> crate::models::ModelInfo {
        let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
        crate::models::ModelInfo {
            id: "t",
            kind: crate::models::Kind::Asr,
            name: "t",
            desc_key: "models.desc.t",
            size_bytes: 16,
            total_bytes: 16 + 6,
            speed: 3,
            quality: 3,
            url: leak(server.url("/main.gguf")),
            filename: "main.gguf",
            sha256: None,
            mmproj: Some(crate::models::ModelFile {
                url: leak(server.url("/mmproj.gguf")),
                filename: "mmproj.gguf",
                size_bytes: 6,
                sha256: None,
            }),
        }
    }

    #[test]
    fn download_model_fetches_every_file_with_cumulative_progress() {
        let server = MockServer::start();
        server.mock(|w, t| {
            w.method(GET).path("/main.gguf");
            t.status(200).header("content-length", "16").body(BODY);
        });
        server.mock(|w, t| {
            w.method(GET).path("/mmproj.gguf");
            t.status(200).header("content-length", "6").body(&BODY[..6]);
        });
        let dir = tempfile::tempdir().unwrap();
        let m = two_file_model(&server);
        let mut seen = vec![];
        download_model(
            &client(),
            dir.path(),
            &m,
            &AtomicBool::new(false),
            &mut |p| seen.push((p.received, p.total)),
        )
        .unwrap();
        assert!(crate::models::installed(dir.path(), &m));
        assert_eq!(
            dir.path()
                .join("asr")
                .join("mmproj.gguf")
                .metadata()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(*seen.last().unwrap(), (22, 22));
        assert!(
            seen.iter().all(|(_, t)| *t == 22),
            "total must be the model total"
        );
        assert!(
            seen.windows(2).all(|w| w[0].0 <= w[1].0),
            "received never decreases"
        );
    }

    #[test]
    fn download_model_skips_files_already_installed() {
        let server = MockServer::start();
        let main = server.mock(|w, t| {
            w.method(GET).path("/main.gguf");
            t.status(200).header("content-length", "16").body(BODY);
        });
        server.mock(|w, t| {
            w.method(GET).path("/mmproj.gguf");
            t.status(200).header("content-length", "6").body(&BODY[..6]);
        });
        let dir = tempfile::tempdir().unwrap();
        let m = two_file_model(&server);
        let main_path = crate::models::model_path(dir.path(), &m);
        std::fs::create_dir_all(main_path.parent().unwrap()).unwrap();
        std::fs::write(&main_path, BODY).unwrap();
        download_model(
            &client(),
            dir.path(),
            &m,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        main.assert_hits(0);
        assert!(crate::models::installed(dir.path(), &m));
    }
}
