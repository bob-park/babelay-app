//! GitHub Releases 의 latest.json 을 보고 새 버전을 찾아 설치한다.
//! 창이 닫혀도 앱은 트레이에서 살아 있으므로 확인은 여기서(Rust) 돈다.

use crate::settings::SettingsState;
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// 마지막 확인에서 찾은 업데이트. 없으면 None.
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
}

impl UpdateInfo {
    fn from_update(u: &Update) -> Self {
        Self {
            version: u.version.clone(),
            notes: u.body.clone().unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct UpdateProgress {
    pub received: u64,
    pub total: Option<u64>,
}

const FIRST_CHECK: Duration = Duration::from_secs(10);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 보관 중인 업데이트 정보. 상태가 아직 등록되지 않았으면(setup 초반) None.
pub fn status(app: &AppHandle) -> Option<UpdateInfo> {
    app.try_state::<UpdateState>()?
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(UpdateInfo::from_update)
}

pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let info = found.as_ref().map(UpdateInfo::from_update);
    *app.state::<UpdateState>().0.lock().unwrap() = found;
    if let Some(i) = &info {
        let _ = app.emit("update-available", i);
    }
    crate::tray::relabel_update(app);
    Ok(info)
}

/// 받고 → 엔진 정리 → 설치 → 재시작. Windows 는 install 이 설치기를 띄우고 프로세스를 끝내므로
/// 정리가 그 앞에 있어야 한다. 실패해도 보관한 Update 는 남겨 다시 시도할 수 있다.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    // 트레이를 연타해도 다운로드·설치는 하나만. 성공 경로는 재시작이라 돌아오지 않는다.
    static INSTALLING: AtomicBool = AtomicBool::new(false);
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("busy".into());
    }
    let fail = |e: String| {
        INSTALLING.store(false, Ordering::SeqCst);
        e
    };
    let update = app
        .state::<UpdateState>()
        .0
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| fail("no_update".to_string()))?;
    let mut received: u64 = 0;
    let mut last_pct = u64::MAX;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                // 청크마다 보내면 수천 번이다. 퍼센트가 바뀔 때만.
                let pct = total.map_or(0, |t| received * 100 / t.max(1));
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| fail(e.to_string()))?;
    crate::shutdown(app);
    update.install(bytes).map_err(|e| fail(e.to_string()))?;
    app.restart()
}

/// setup 에서 한 번. 설정을 매 주기마다 다시 읽으므로 토글이 바로 반영된다.
/// 오프라인은 정상 상황이라 실패는 로그만 남긴다.
pub fn spawn_periodic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if app.state::<SettingsState>().get().general.auto_update {
                if let Err(e) = check(&app).await {
                    eprintln!("babelay update: check failed: {e}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// 트레이 항목 하나가 "확인" 과 "설치" 를 겸한다. 오류는 이벤트로 메인 창에 보낸다.
pub fn on_tray_click(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let r = if status(&app).is_some() {
            install(&app).await
        } else {
            check(&app).await.map(|_| ())
        };
        if let Err(e) = r {
            eprintln!("babelay update: {e}");
            let _ = app.emit("update-error", e);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_serializes_with_frontend_keys() {
        let v = serde_json::to_value(UpdateInfo {
            version: "0.3.0".into(),
            notes: "fix".into(),
        })
        .unwrap();
        assert_eq!(v, serde_json::json!({ "version": "0.3.0", "notes": "fix" }));
        let p = serde_json::to_value(UpdateProgress {
            received: 5,
            total: None,
        })
        .unwrap();
        assert_eq!(p, serde_json::json!({ "received": 5, "total": null }));
    }
}
