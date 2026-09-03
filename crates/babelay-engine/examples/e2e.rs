//! End-to-end check without the GUI: real tap + real whisper. Usage:
//!   BABELAY_TEST_MODEL=<ggml-*.bin> mise exec -- cargo run -p babelay-engine --features metal --example e2e
use babelay_engine::engine::{start_default, EngineConfig, EngineEvent};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

fn main() {
    let model = std::env::var("BABELAY_TEST_MODEL").expect("BABELAY_TEST_MODEL");
    let (tx, rx) = mpsc::channel();
    let cfg = EngineConfig {
        model_path: model.into(),
        model_id: "test".into(),
        use_gpu: true,
        source_lang: Some("en".into()),
        tgt_lang: None,
    };
    let handle = start_default(cfg, None, tx).expect("engine start");
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(500));
        let _ = std::process::Command::new("say")
            .args([
                "-r",
                "170",
                "The quick brown fox jumps over the lazy dog. Babelay shows live subtitles.",
            ])
            .status();
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut finals = 0;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(ev) => {
                println!("{}", serde_json::to_string(&ev).unwrap());
                if matches!(ev, EngineEvent::Final { .. }) {
                    finals += 1;
                    if finals >= 2 {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    handle.stop();
    for ev in rx.try_iter() {
        println!("{}", serde_json::to_string(&ev).unwrap());
    }
    println!("finals={finals}");
    std::process::exit(if finals > 0 { 0 } else { 1 });
}
