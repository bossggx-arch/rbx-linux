use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const SOBER_APP_ID: &str = "org.vinegarhq.Sober";
const FLATHUB_REPO_URL: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

/// Check if `flatpak` CLI is available on the system
pub fn flatpak_available() -> bool {
    Command::new("flatpak")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if Sober (`org.vinegarhq.Sober`) is installed via Flatpak
pub fn sober_installed() -> bool {
    Command::new("flatpak")
        .args(["info", SOBER_APP_ID])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Install Sober from Flathub for the current user
pub fn install_sober() -> Result<(), String> {
    if !flatpak_available() {
        return Err(concat!(
            "ไม่พบคำสั่ง flatpak บนเครื่อง กรุณาติดตั้ง Flatpak ก่อน:\n",
            "  - Ubuntu/Debian: sudo apt install flatpak\n",
            "  - Arch Linux:    sudo pacman -S flatpak\n",
            "  - Fedora:        sudo dnf install flatpak"
        )
        .to_string());
    }

    println!("[Sober] กำลังเพิ่ม Flathub repository (user)...");
    let remote_status = Command::new("flatpak")
        .args([
            "remote-add",
            "--if-not-exists",
            "--user",
            "flathub",
            FLATHUB_REPO_URL,
        ])
        .status()
        .map_err(|e| format!("ไม่สามารถรันคำสั่ง flatpak remote-add ได้: {}", e))?;

    if !remote_status.success() {
        return Err("เพิ่ม Flathub repository ไม่สำเร็จ".to_string());
    }

    println!("[Sober] กำลังติดตั้ง Sober ({}) ผ่าน Flatpak...", SOBER_APP_ID);
    let install_status = Command::new("flatpak")
        .args(["install", "--user", "-y", "flathub", SOBER_APP_ID])
        .status()
        .map_err(|e| format!("ไม่สามารถรันคำสั่ง flatpak install ได้: {}", e))?;

    if !install_status.success() {
        return Err(format!("ติดตั้ง {} ผ่าน Flatpak ไม่สำเร็จ", SOBER_APP_ID));
    }

    println!("[Sober] ติดตั้ง Sober เสร็จสมบูรณ์!");
    Ok(())
}

/// Build arguments passed to `flatpak` to launch Sober with optional URI/arguments
pub fn sober_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(2 + args.len());
    out.push("run".to_string());
    out.push(SOBER_APP_ID.to_string());
    out.extend(args.iter().cloned());
    out
}

/// Merge FastFlags into a parsed Sober config JSON object under `"fflags"`
pub fn merge_fflags(
    config: &mut serde_json::Value,
    fflags: &BTreeMap<String, serde_json::Value>,
) {
    if !config.is_object() {
        *config = serde_json::json!({});
    }

    let root = config.as_object_mut().expect("config must be an object");
    if !root.get("fflags").map(|v| v.is_object()).unwrap_or(false) {
        root.insert("fflags".to_string(), serde_json::json!({}));
    }

    if let Some(fflags_obj) = root.get_mut("fflags").and_then(|v| v.as_object_mut()) {
        for (k, v) in fflags {
            fflags_obj.insert(k.clone(), v.clone());
        }
    }
}

fn sober_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".var")
            .join("app")
            .join(SOBER_APP_ID)
            .join("config")
            .join("sober")
            .join("config.json")
    })
}

/// Apply FastFlags to Sober's existing config.json without creating it if missing
pub fn apply_fflags(fflags: &BTreeMap<String, serde_json::Value>) -> Result<(), String> {
    let Some(path) = sober_config_path() else {
        return Ok(());
    };

    if !path.exists() {
        return Ok(());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("ไม่สามารถอ่านไฟล์ตั้งค่า Sober {:?}: {}", path, e))?;

    let mut parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "[คำเตือน] ไม่สามารถอ่าน JSON ของ Sober ({:?}) ได้ จึงข้ามการอัปเดต FastFlags: {}",
                path, e
            );
            return Ok(());
        }
    };

    merge_fflags(&mut parsed, fflags);

    let pretty = serde_json::to_string_pretty(&parsed)
        .map_err(|e| format!("แปลง JSON สำหรับ Sober ไม่สำเร็จ: {}", e))?;
    fs::write(&path, pretty)
        .map_err(|e| format!("บันทึกไฟล์ตั้งค่า Sober {:?} ไม่สำเร็จ: {}", path, e))?;

    Ok(())
}

/// Launch Roblox Player online via Sober (installing it automatically if missing)
pub fn launch(
    args: &[String],
    fflags: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    if !sober_installed() {
        println!("[Sober] ยังไม่พบ Sober บนเครื่อง กำลังติดตั้งให้อัตโนมัติ...");
        install_sober()?;
    }

    if let Err(e) = apply_fflags(fflags) {
        eprintln!("[คำเตือน] {}", e);
    }

    let cmd_args = sober_args(args);
    println!("[Sober] กำลังเปิด Roblox ผ่าน Sober ({})...", SOBER_APP_ID);
    let status = Command::new("flatpak")
        .args(&cmd_args)
        .status()
        .map_err(|e| format!("ไม่สามารถเปิด Sober ผ่าน flatpak ได้: {}", e))?;

    if !status.success() {
        println!(
            "[Sober] โปรแกรมจบการทำงานด้วยรหัสสถานะ: {:?}",
            status.code()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sober_args_preserves_uri() {
        let input = vec!["roblox://placeId=1818".to_string()];
        assert_eq!(
            sober_args(&input),
            vec!["run", "org.vinegarhq.Sober", "roblox://placeId=1818"]
        );
    }

    #[test]
    fn test_merge_fflags_adds_fflags_section() {
        let mut config = serde_json::json!({"other": 1});
        let mut fflags = BTreeMap::new();
        fflags.insert("FFlagX".to_string(), serde_json::json!(true));

        merge_fflags(&mut config, &fflags);

        assert_eq!(
            config,
            serde_json::json!({
                "other": 1,
                "fflags": {
                    "FFlagX": true
                }
            })
        );
    }

    #[test]
    fn test_merge_fflags_preserves_existing_flags() {
        let mut config = serde_json::json!({"fflags": {"A": 1}});
        let mut fflags = BTreeMap::new();
        fflags.insert("B".to_string(), serde_json::json!(false));

        merge_fflags(&mut config, &fflags);

        assert_eq!(
            config,
            serde_json::json!({
                "fflags": {
                    "A": 1,
                    "B": false
                }
            })
        );
    }
}
