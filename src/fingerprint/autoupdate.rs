// Browser version auto-updater
// Fetches latest versions from official APIs and caches them locally

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const UPDATE_THRESHOLD_DAYS: i64 = 30;
const SAFARI_STALE_THRESHOLD_DAYS: i64 = 180; // Safari updates quarterly

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BrowserVersions {
    pub last_updated: DateTime<Utc>,
    pub safari_last_checked: DateTime<Utc>,
    pub chrome: Vec<(String, String)>,
    pub firefox: Vec<String>,
    pub safari: Vec<(String, String)>,
}

impl BrowserVersions {
    /// Load versions from cache or fetch updates if stale
    #[must_use]
    pub fn load_or_update() -> Self {
        let config_path = Self::config_path();

        // Try to load existing config
        if let Ok(config) = Self::load_from_file(&config_path) {
            // Check if stale (>30 days old)
            if config.is_stale() {
                eprintln!(
                    "🔄 Browser versions outdated ({} days old), updating...",
                    (Utc::now() - config.last_updated).num_days()
                );

                match config.fetch_and_update() {
                    Ok(updated) => {
                        if let Err(e) = updated.save_to_file(&config_path) {
                            eprintln!("⚠️  Failed to save updates: {e}");
                        }
                        updated.check_safari_staleness();
                        return updated;
                    }
                    Err(e) => {
                        eprintln!("⚠️  Update failed ({e}), using cached versions");
                        config.check_safari_staleness();
                    }
                }
            }
            return config;
        }

        // No config exists, create from defaults and try to update
        eprintln!("🔄 Initializing browser versions...");
        let config = Self::default();

        match config.fetch_and_update() {
            Ok(updated) => {
                if let Err(e) = updated.save_to_file(&config_path) {
                    eprintln!("⚠️  Failed to save initial config: {e}");
                    return config;
                }
                eprintln!("✅ Browser versions initialized");
                updated
            }
            Err(e) => {
                eprintln!("⚠️  Failed to fetch initial versions ({e}), using defaults");
                config
            }
        }
    }

    fn is_stale(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.last_updated);
        age > Duration::days(UPDATE_THRESHOLD_DAYS)
    }

    fn is_safari_critically_stale(&self) -> bool {
        let safari_age = Utc::now().signed_duration_since(self.safari_last_checked);
        safari_age > Duration::days(SAFARI_STALE_THRESHOLD_DAYS)
    }

    fn check_safari_staleness(&self) {
        if self.is_safari_critically_stale() {
            let days = (Utc::now() - self.safari_last_checked).num_days();
            eprintln!("⚠️  Safari versions are {days} days old (>6 months)");
            eprintln!("   Check: https://developer.apple.com/documentation/safari-release-notes");
            eprintln!("   Or edit: {:?}", Self::config_path());
        }
    }

    fn fetch_and_update(&self) -> Result<Self, Box<dyn std::error::Error>> {
        // Fetch Chrome and Firefox (auto-update)
        let chrome = Self::fetch_chrome_versions().unwrap_or_else(|e| {
            eprintln!("⚠️  Chrome update failed ({e}), using cached");
            self.chrome.clone()
        });

        let firefox = Self::fetch_firefox_versions().unwrap_or_else(|e| {
            eprintln!("⚠️  Firefox update failed ({e}), using cached");
            self.firefox.clone()
        });

        // Safari: Try community list, fall back to cached
        let (safari, safari_updated) = match Self::fetch_safari_from_community() {
            Ok(versions) => {
                eprintln!("✅ Safari: Updated from community list");
                (versions, Utc::now())
            }
            Err(_) => {
                // Keep existing Safari versions and timestamp
                (self.safari.clone(), self.safari_last_checked)
            }
        };

        Ok(BrowserVersions {
            last_updated: Utc::now(),
            safari_last_checked: safari_updated,
            chrome,
            firefox,
            safari,
        })
    }

    fn fetch_chrome_versions() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        // Google's official Chrome version API (replaced defunct Omahaproxy)
        let url = "https://versionhistory.googleapis.com/v1/chrome/platforms/mac/channels/stable/versions/all/releases?filter=endtime=none";

        eprintln!("🔍 Fetching Chrome versions...");
        let resp: serde_json::Value = reqwest::blocking::get(url)?.error_for_status()?.json()?;

        let mut versions = Vec::new();
        if let Some(releases) = resp["releases"].as_array() {
            eprintln!("   Found {} releases in API response", releases.len());
            for release in releases {
                if let Some(full) = release["version"].as_str() {
                    let major = full.split('.').next().unwrap_or("0");
                    versions.push((major.to_string(), format!("{major}.0.0.0")));
                }
            }
        } else {
            return Err("No 'releases' array in API response".into());
        }

        // Deduplicate and keep latest 5
        versions.sort_by(|a, b| {
            b.0.parse::<u32>()
                .unwrap_or(0)
                .cmp(&a.0.parse::<u32>().unwrap_or(0))
        });
        versions.dedup_by(|a, b| a.0 == b.0);
        versions.truncate(5);

        if versions.is_empty() {
            return Err("No Chrome versions found".into());
        }

        eprintln!(
            "✅ Chrome: {} versions ({} to {})",
            versions.len(),
            versions[0].0,
            versions.last().unwrap().0
        );
        Ok(versions)
    }

    fn fetch_firefox_versions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = "https://product-details.mozilla.org/1.0/firefox_versions.json";
        let resp: serde_json::Value = reqwest::blocking::get(url)?.error_for_status()?.json()?;

        let latest = resp["LATEST_FIREFOX_VERSION"]
            .as_str()
            .ok_or("Missing LATEST_FIREFOX_VERSION")?
            .split('.')
            .next()
            .ok_or("Invalid version format")?
            .parse::<u32>()?;

        // Generate last 4 versions
        let versions: Vec<String> = (0..4)
            .map(|i| format!("{}.0", latest.saturating_sub(i)))
            .collect();

        eprintln!(
            "✅ Firefox: {} versions ({} to {})",
            versions.len(),
            versions[0],
            versions.last().unwrap()
        );
        Ok(versions)
    }

    fn fetch_safari_from_community() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        // Future: Implement community-maintained list
        // For now, return error to use cached versions
        Err("Community list not yet implemented".into())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("microfetch")
            .join("versions.json")
    }

    fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: BrowserVersions = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for BrowserVersions {
    fn default() -> Self {
        let now = Utc::now();
        BrowserVersions {
            last_updated: now,
            safari_last_checked: now,
            chrome: vec![
                ("131".into(), "131.0.0.0".into()),
                ("130".into(), "130.0.0.0".into()),
                ("129".into(), "129.0.0.0".into()),
                ("128".into(), "128.0.0.0".into()),
                ("127".into(), "127.0.0.0".into()),
            ],
            firefox: vec![
                "134.0".into(),
                "133.0".into(),
                "132.0".into(),
                "131.0".into(),
            ],
            safari: vec![
                ("18.2".into(), "619.1.15".into()),
                ("18.1".into(), "619.1.15".into()),
                ("18.0".into(), "618.1.15".into()),
                ("17.6".into(), "605.1.15".into()),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_staleness() {
        let old = BrowserVersions {
            last_updated: Utc::now() - Duration::days(31),
            safari_last_checked: Utc::now(),
            ..Default::default()
        };
        assert!(old.is_stale());

        let fresh = BrowserVersions::default();
        assert!(!fresh.is_stale());
    }

    #[test]
    fn test_safari_staleness() {
        let old_safari = BrowserVersions {
            last_updated: Utc::now(),
            safari_last_checked: Utc::now() - Duration::days(185),
            ..Default::default()
        };
        assert!(old_safari.is_safari_critically_stale());
    }

    #[test]
    fn test_fetch_chrome_versions() {
        // Network test - may fail if offline
        if let Ok(versions) = BrowserVersions::fetch_chrome_versions() {
            assert!(!versions.is_empty());
            assert!(versions.len() <= 5);
            // Major version should be >= 131
            let major: u32 = versions[0].0.parse().unwrap();
            assert!(major >= 131);
        }
    }

    #[test]
    fn test_fetch_firefox_versions() {
        // Network test - may fail if offline
        if let Ok(versions) = BrowserVersions::fetch_firefox_versions() {
            assert_eq!(versions.len(), 4);
            // Version should be >= 134
            let major: u32 = versions[0].split('.').next().unwrap().parse().unwrap();
            assert!(major >= 134);
        }
    }
}
