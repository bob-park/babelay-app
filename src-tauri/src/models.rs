use crate::settings::{Settings, SettingsState};
use babelay_engine::{
    download::{download, DownloadError, Progress},
    models::{find, installed, model_path, ModelInfo, BALANCED},
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone, Copy, Debug)]
pub struct DownloadProgress {
    pub received: u64,
    pub total: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ModelStatus {
    pub info: ModelInfo,
    pub installed: bool,
    pub in_use: bool,
    pub balanced: bool,
    pub download: Option<DownloadProgress>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DownloadEvent {
    pub id: String,
    pub received: u64,
    pub total: u64,
    pub state: &'static str, // downloading | done | error | cancelled
    pub message: Option<String>,
}

struct Active {
    id: String,
    cancel: Arc<AtomicBool>,
    progress: DownloadProgress,
}

#[derive(Default)]
pub struct Downloads {
    active: Mutex<Option<Active>>,
}

pub fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("models"))
        .map_err(|e| e.to_string())
}

fn in_use(settings: &Settings, m: &ModelInfo) -> bool {
    settings.asr.model_id == m.id || settings.translation.local_model == m.id
}

pub fn list(app: &AppHandle) -> Result<Vec<ModelStatus>, String> {
    let dir = models_dir(app)?;
    let settings = app.state::<SettingsState>().get();
    let downloads = app.state::<Downloads>();
    let active = downloads.active.lock().unwrap();
    Ok(babelay_engine::models::REGISTRY
        .iter()
        .map(|m| ModelStatus {
            info: m.clone(),
            installed: installed(&dir, m),
            in_use: in_use(&settings, m),
            balanced: m.id == BALANCED.asr || m.id == BALANCED.llm,
            download: active
                .as_ref()
                .filter(|a| a.id == m.id)
                // 망가진 .part 로 received 가 total 을 넘으면 UI 막대가 튄다
                .map(|a| DownloadProgress {
                    received: a.progress.received.min(a.progress.total),
                    total: a.progress.total,
                }),
        })
        .collect())
}

pub fn start(app: &AppHandle, id: &str) -> Result<(), String> {
    let m = find(id).ok_or("unknown model")?;
    let dir = models_dir(app)?;
    // 30초간 수신이 없으면 끊는다. 블로킹 클라이언트는 이 값을 read 마다 적용한다.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let downloads = app.state::<Downloads>();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut active = downloads.active.lock().unwrap();
        if active.is_some() {
            return Err("busy".into());
        }
        *active = Some(Active {
            id: id.to_string(),
            cancel: cancel.clone(),
            progress: DownloadProgress {
                received: 0,
                total: m.size_bytes,
            },
        });
    }
    let app2 = app.clone();
    let id_owned = id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let dest = model_path(&dir, m);
        let mut last = Instant::now() - Duration::from_secs(1);
        let mut on_progress = |p: Progress| {
            let progress = DownloadProgress {
                received: p.received,
                total: p.total,
            };
            if let Some(a) = app2.state::<Downloads>().active.lock().unwrap().as_mut() {
                a.progress = progress;
            }
            if last.elapsed() >= Duration::from_millis(200) || p.received == p.total {
                last = Instant::now();
                let _ = app2.emit(
                    "model-download",
                    DownloadEvent {
                        id: id_owned.clone(),
                        received: p.received,
                        total: p.total,
                        state: "downloading",
                        message: None,
                    },
                );
            }
        };
        let result = download(
            &client,
            m.url,
            &dest,
            m.size_bytes,
            m.sha256,
            &cancel,
            &mut on_progress,
        );
        let (state, message) = match &result {
            Ok(()) => ("done", None),
            Err(DownloadError::Cancelled) => ("cancelled", None),
            Err(e) => ("error", Some(e.to_string())),
        };
        let progress = app2
            .state::<Downloads>()
            .active
            .lock()
            .unwrap()
            .take()
            .map(|a| a.progress)
            .unwrap_or(DownloadProgress {
                received: 0,
                total: m.size_bytes,
            });
        let _ = app2.emit(
            "model-download",
            DownloadEvent {
                id: id_owned,
                received: progress.received,
                total: progress.total,
                state,
                message,
            },
        );
    });
    Ok(())
}

pub fn cancel(app: &AppHandle, id: &str) -> Result<(), String> {
    let downloads = app.state::<Downloads>();
    let active = downloads.active.lock().unwrap();
    match active.as_ref() {
        Some(a) if a.id == id => {
            a.cancel.store(true, Ordering::Relaxed);
            Ok(())
        }
        _ => Err("not downloading".into()),
    }
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    let m = find(id).ok_or("unknown model")?;
    let settings = app.state::<SettingsState>().get();
    if in_use(&settings, m) {
        return Err("in_use".into());
    }
    if app
        .state::<Downloads>()
        .active
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|a| a.id == id)
    {
        return Err("busy".into());
    }
    let path = model_path(&models_dir(app)?, m);
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    for p in [path, PathBuf::from(part)] {
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
