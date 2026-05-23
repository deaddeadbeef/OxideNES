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
        Self::parse_latest_release(&body, current_version)
    }

    fn parse_latest_release(body: &str, current_version: &str) -> Option<UpdateInfo> {
        let json: serde_json::Value = serde_json::from_str(body).ok()?;
        let tag = json.get("tag_name")?.as_str()?;
        let latest = semver::Version::parse(tag.trim_start_matches('v')).ok()?;
        let current = semver::Version::parse(current_version).ok()?;

        if latest > current {
            let download_url = json
                .get("assets")
                .and_then(|assets| assets.as_array())
                .and_then(|assets| {
                    assets.iter().find_map(|asset| {
                        asset
                            .get("browser_download_url")
                            .and_then(|url| url.as_str())
                            .filter(|url| !url.is_empty())
                    })
                })
                .unwrap_or("")
                .to_string();

            Some(UpdateInfo {
                version: tag.to_string(),
                download_url,
                changelog: json
                    .get("body")
                    .and_then(|body| body.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        } else {
            None
        }
    }

    pub fn get_update(&self) -> Option<UpdateInfo> {
        self.update_available.lock().ok()?.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_latest_release_rejects_malformed_json() {
        assert!(Updater::parse_latest_release("not json", "0.3.1").is_none());
    }

    #[test]
    fn parse_latest_release_rejects_missing_or_invalid_tag() {
        assert!(Updater::parse_latest_release(r#"{"body":"notes"}"#, "0.3.1").is_none());
        assert!(Updater::parse_latest_release(r#"{"tag_name":"latest"}"#, "0.3.1").is_none());
    }

    #[test]
    fn parse_latest_release_ignores_current_or_older_version() {
        let body = r#"{"tag_name":"v0.3.1","assets":[],"body":"notes"}"#;
        assert!(Updater::parse_latest_release(body, "0.3.1").is_none());

        let body = r#"{"tag_name":"v0.3.0","assets":[],"body":"notes"}"#;
        assert!(Updater::parse_latest_release(body, "0.3.1").is_none());
    }

    #[test]
    fn parse_latest_release_handles_missing_assets_without_panic() {
        let body = r#"{"tag_name":"v0.3.2","body":"notes"}"#;
        let info = Updater::parse_latest_release(body, "0.3.1").expect("new version");

        assert_eq!(info.version, "v0.3.2");
        assert_eq!(info.download_url, "");
        assert_eq!(info.changelog, "notes");
    }

    #[test]
    fn parse_latest_release_uses_first_non_empty_asset_url() {
        let body = r#"{
            "tag_name":"v0.3.2",
            "body":"notes",
            "assets":[
                {"browser_download_url":""},
                {"browser_download_url":"https://example.invalid/oxidenes.exe"}
            ]
        }"#;
        let info = Updater::parse_latest_release(body, "0.3.1").expect("new version");

        assert_eq!(info.download_url, "https://example.invalid/oxidenes.exe");
    }
}
