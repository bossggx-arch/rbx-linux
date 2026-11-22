use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Path or command name for Wine binary (e.g., "wine", "wine64", or custom path)
    pub wine_binary: String,
    /// Optional path to a Proton installation directory (if set, uses Proton instead of plain Wine)
    pub proton_dir: Option<String>,
    /// Graphics renderer: "D3D11", "Vulkan", or "OpenGL"
    pub renderer: String,
    /// Enable DXVK for Direct3D 11 -> Vulkan translation
    pub dxvk: bool,
    /// Enable Wine Esync
    pub esync: bool,
    /// Enable Wine Fsync
    pub fsync: bool,
    /// Roblox deployment channel (default: "LIVE")
    pub channel: String,
    /// Optional pinned Studio version (e.g. "version-xxxxxxxxxxxxxxxx")
    pub forced_studio_version: Option<String>,
    /// Optional pinned Player version (e.g. "version-xxxxxxxxxxxxxxxx")
    pub forced_player_version: Option<String>,
    /// Custom environment variables passed to Wine
    pub env: BTreeMap<String, String>,
    /// Custom FastFlags merged into ClientSettings/ClientAppSettings.json
    pub fflags_studio: BTreeMap<String, serde_json::Value>,
    pub fflags_player: BTreeMap<String, serde_json::Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut fflags_studio = BTreeMap::new();
        fflags_studio.insert(
            "FFlagDebugGraphicsPreferD3D11".to_string(),
            serde_json::Value::Bool(true),
        );

        let mut fflags_player = BTreeMap::new();
        fflags_player.insert(
            "FFlagDebugGraphicsPreferD3D11".to_string(),
            serde_json::Value::Bool(true),
        );

        Self {
            wine_binary: "wine".to_string(),
            proton_dir: None,
            renderer: "D3D11".to_string(),
            dxvk: true,
            esync: true,
            fsync: true,
            channel: "LIVE".to_string(),
            forced_studio_version: None,
            forced_player_version: None,
            env: BTreeMap::new(),
            fflags_studio,
            fflags_player,
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rbx-linux")
    }

    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rbx-linux")
    }

    pub fn prefix_dir() -> PathBuf {
        Self::data_dir().join("pfx")
    }

    pub fn versions_dir() -> PathBuf {
        Self::data_dir().join("versions")
    }

    pub fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rbx-linux")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::config_file();
        if !path.exists() {
            let default_cfg = Self::default();
            let _ = default_cfg.save();
            return Ok(default_cfg);
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("ไม่สามารถอ่านไฟล์ config {:?}: {}", path, e))?;
        toml::from_str(&raw)
            .map_err(|e| format!("รูปแบบไฟล์ config.toml ไม่ถูกต้อง: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)
            .map_err(|e| format!("สร้างโฟลเดอร์ config ไม่สำเร็จ: {}", e))?;
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| format!("แปลง config เป็น TOML ไม่สำเร็จ: {}", e))?;
        fs::write(Self::config_file(), toml_str)
            .map_err(|e| format!("บันทึกไฟล์ config ไม่สำเร็จ: {}", e))
    }

    pub fn merged_fflags(&self, is_studio: bool) -> BTreeMap<String, serde_json::Value> {
        let mut map = if is_studio {
            self.fflags_studio.clone()
        } else {
            self.fflags_player.clone()
        };

        // Apply renderer choice automatically
        match self.renderer.to_uppercase().as_str() {
            "VULKAN" => {
                map.insert("FFlagDebugGraphicsPreferVulkan".into(), serde_json::Value::Bool(true));
                map.insert("FFlagDebugGraphicsPreferD3D11".into(), serde_json::Value::Bool(false));
                map.insert("FFlagDebugGraphicsDisableDirect3D11".into(), serde_json::Value::Bool(true));
            }
            "OPENGL" => {
                map.insert("FFlagDebugGraphicsPreferOpenGL".into(), serde_json::Value::Bool(true));
                map.insert("FFlagDebugGraphicsPreferD3D11".into(), serde_json::Value::Bool(false));
                map.insert("FFlagDebugGraphicsDisableDirect3D11".into(), serde_json::Value::Bool(true));
            }
            _ => {
                // Default D3D11 (works best with DXVK on Linux)
                map.insert("FFlagDebugGraphicsPreferD3D11".into(), serde_json::Value::Bool(true));
            }
        }
        map
    }
}
