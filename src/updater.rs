use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub changelog: String,
}

pub struct Updater {
    pub update_available: Arc<Mutex<Option<UpdateInfo>>>,
    pub checking: Arc<Mutex<bool>>,
}

impl Updater {
    pub fn new() -> Self {
        Updater {
            update_available: Arc::new(Mutex::new(None)),
            checking: Arc::new(Mutex::new(false)),
        }
    }
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn check_async(&self) {
        let available = self.update_available.clone();
        let checking = self.checking.clone();
        if let Ok(mut c) = checking.lock() {
            *c = true;
        }

        thread::spawn(move || {
            let result = Self::check_github();
            if let Ok(mut a) = available.lock() {
                *a = result;
            }
            if let Ok(mut c) = checking.lock() {
                *c = false;
            }
        });
    }

    fn check_github() -> Option<UpdateInfo> {
        let current_version = env!("CARGO_PKG_VERSION");
        let resp = ureq::get("https://api.github.com/repos/deaddeadbeef/OxideNES/releases/latest")
            .set("User-Agent", "oxidenes")
            .call()
            .ok()?;
        let body: String = resp.into_string().ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;
        let tag = json["tag_name"].as_str()?;
        let latest = semver::Version::parse(tag.trim_start_matches('v')).ok()?;
        let current = semver::Version::parse(current_version).ok()?;

        if latest > current {
            Some(UpdateInfo {
                version: tag.to_string(),
                download_url: json["assets"][0]["browser_download_url"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                changelog: json["body"].as_str().unwrap_or("").to_string(),
            })
        } else {
            None
        }
    }

    pub fn get_update(&self) -> Option<UpdateInfo> {
        self.update_available.lock().ok()?.clone()
    }
}
