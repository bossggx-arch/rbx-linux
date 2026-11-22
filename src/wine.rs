use crate::config::AppConfig;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct WineRunner<'a> {
    pub cfg: &'a AppConfig,
}

impl<'a> WineRunner<'a> {
    pub fn new(cfg: &'a AppConfig) -> Self {
        Self { cfg }
    }

    /// Build a configured `Command` for running a Windows program in the managed Wine/Proton prefix
    pub fn build_command(&self, exe_or_tool: &str) -> Command {
        let prefix = AppConfig::prefix_dir();

        if let Some(proton_dir) = &self.cfg.proton_dir {
            let proton_bin = Path::new(proton_dir).join("proton");
            let mut cmd = Command::new(proton_bin);
            cmd.arg("run").arg(exe_or_tool);
            cmd.env("STEAM_COMPAT_DATA_PATH", &prefix);
            cmd.env(
                "STEAM_COMPAT_CLIENT_INSTALL_PATH",
                dirs::home_dir().unwrap_or_default().join(".steam/steam"),
            );
            self.apply_common_env(&mut cmd, &prefix.join("pfx"));
            cmd
        } else {
            let mut cmd = Command::new(&self.cfg.wine_binary);
            cmd.arg(exe_or_tool);
            self.apply_common_env(&mut cmd, &prefix);
            cmd
        }
    }

    fn apply_common_env(&self, cmd: &mut Command, prefix_path: &Path) {
        cmd.env("WINEPREFIX", prefix_path);
        cmd.env("WINEARCH", "win64");
        cmd.env("WINEDEBUG", "-all");

        if self.cfg.esync {
            cmd.env("WINEESYNC", "1");
        }
        if self.cfg.fsync {
            cmd.env("WINEFSYNC", "1");
        }

        // Disable Wine gecko/mono prompts on prefix creation and enable DXVK overrides if configured
        let mut dll_overrides = vec!["winemenubuilder.exe=d".to_string(), "mscoree=d".to_string()];
        if self.cfg.dxvk {
            dll_overrides.push("d3d11=n,b".to_string());
            dll_overrides.push("dxgi=n,b".to_string());
            dll_overrides.push("d3d10core=n,b".to_string());
            dll_overrides.push("d3d9=n,b".to_string());
            cmd.env("DXVK_LOG_LEVEL", "none");
        }
        cmd.env("WINEDLLOVERRIDES", dll_overrides.join(";"));

        for (k, v) in &self.cfg.env {
            cmd.env(k, v);
        }
    }

    /// Initialize Wine prefix, set Windows version to Windows 10, and configure DXVK if available
    pub fn init_prefix(&self) -> Result<(), String> {
        let prefix = AppConfig::prefix_dir();
        fs::create_dir_all(&prefix)
            .map_err(|e| format!("สร้างโฟลเดอร์ Wine Prefix ไม่สำเร็จ: {}", e))?;

        println!("[Wine] กำลังเตรียม Wine Prefix ที่ {:?}...", prefix);

        // Run wineboot -u
        let status = self
            .build_command("wineboot")
            .arg("-u")
            .status()
            .map_err(|e| {
                format!(
                    "ไม่พบคำสั่ง '{}' กรุณาติดตั้ง Wine บน Linux ก่อน (เช่น sudo apt install wine หรือ sudo pacman -S wine): {}",
                    self.cfg.wine_binary, e
                )
            })?;

        if !status.success() {
            return Err("การเริ่มต้น Wine Prefix (wineboot -u) ล้มเหลว".to_string());
        }

        // Set Windows version to Windows 10 via winecfg
        println!("[Wine] ตั้งค่าเวอร์ชัน Windows ของ Prefix เป็น Windows 10...");
        let _ = self
            .build_command("winecfg")
            .args(["-v", "win10"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if self.cfg.dxvk {
            self.try_setup_dxvk(&prefix);
        }

        println!("[Wine] เตรียม Wine Prefix เสร็จสมบูรณ์!");
        Ok(())
    }

    fn try_setup_dxvk(&self, prefix: &Path) {
        // Check if system `setup_dxvk` script is installed (common on Arch/Fedora/Ubuntu dxvk package)
        let check = Command::new("setup_dxvk")
            .arg("install")
            .env("WINEPREFIX", prefix)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match check {
            Ok(s) if s.success() => {
                println!("[DXVK] ติดตั้ง DXVK เข้าสู่ Wine Prefix เรียบร้อยแล้ว (ผ่าน setup_dxvk)");
            }
            _ => {
                println!(
                    "[DXVK] เปิดใช้งาน DXVK DLL Overrides แล้ว (หากต้องการติดตั้งไฟล์ DXVK เพิ่มเติม สามารถรัน `winetricks dxvk` หรือติดตั้งแพ็กเกจ `dxvk` ของ Linux distro ได้)"
                );
            }
        }
    }

    /// Launch a Windows executable with optional arguments and protocol URI parsing
    pub fn launch(&self, exe_path: &Path, extra_args: &[String]) -> Result<(), String> {
        if !exe_path.exists() {
            return Err(format!("ไม่พบไฟล์โปรแกรม: {:?}", exe_path));
        }

        let prefix = AppConfig::prefix_dir();
        if !prefix.join("system.reg").exists() && !prefix.join("pfx/system.reg").exists() {
            self.init_prefix()?;
        }

        let parsed_args = translate_protocol_args(extra_args);
        let exe_str = exe_path.to_string_lossy().to_string();

        println!("[Launch] กำลังรัน {:?} ผ่าน Wine/Proton...", exe_path.file_name().unwrap_or_default());
        if !parsed_args.is_empty() {
            println!("[Launch] Arguments: {:?}", parsed_args);
        }

        let mut cmd = self.build_command(&exe_str);
        if let Some(parent) = exe_path.parent() {
            cmd.current_dir(parent);
        }
        cmd.args(&parsed_args);

        let status = cmd
            .status()
            .map_err(|e| format!("ไม่สามารถรันโปรแกรมผ่าน Wine ได้: {}", e))?;

        if !status.success() {
            println!(
                "[Launch] โปรแกรมจบการทำงานด้วยรหัสสถานะ: {:?}",
                status.code()
            );
        }
        Ok(())
    }

    /// Kill all running processes inside the managed Wine prefix
    pub fn kill_prefix(&self) -> Result<(), String> {
        let prefix = AppConfig::prefix_dir();
        let status = Command::new("wineserver")
            .arg("-k")
            .env("WINEPREFIX", &prefix)
            .status()
            .map_err(|e| format!("รันคำสั่ง wineserver -k ไม่สำเร็จ: {}", e))?;

        if status.success() {
            println!("[Wine] ปิดโปรเซสทั้งหมดใน Wine Prefix เรียบร้อยแล้ว");
        }
        Ok(())
    }
}

/// Converts `roblox-studio:1+launchmode:edit+task:EditPlace+placeId:123...` or `roblox-player:...`
/// into command-line arguments understood by RobloxStudioBeta.exe / RobloxPlayerBeta.exe.
pub fn translate_protocol_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();

    for arg in args {
        if arg.starts_with("roblox-studio:") && !arg.starts_with("roblox-studio://") {
            // Studio protocol string format: key:value pairs separated by '+'
            let mut place_id: Option<String> = None;
            let mut universe_id: Option<String> = None;
            let mut task: Option<String> = None;

            for part in arg.split('+') {
                if let Some((k, v)) = part.split_once(':') {
                    match k {
                        "placeId" if !v.is_empty() => place_id = Some(v.to_string()),
                        "universeId" if !v.is_empty() => universe_id = Some(v.to_string()),
                        "task" if !v.is_empty() => task = Some(v.to_string()),
                        _ => {}
                    }
                }
            }

            if let (Some(t), Some(p)) = (&task, &place_id) {
                out.push("-task".to_string());
                out.push(t.clone());
                out.push("-placeId".to_string());
                out.push(p.clone());
                if let Some(u) = universe_id {
                    out.push("-universeId".to_string());
                    out.push(u);
                }
            } else {
                out.push("-protocolString".to_string());
                out.push(arg.clone());
            }
        } else if arg.starts_with("roblox-studio://") {
            out.push("-url".to_string());
            out.push(arg.clone());
        } else {
            out.push(arg.clone());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_studio_edit_uri() {
        let input = vec![
            "roblox-studio:1+launchmode:edit+task:EditPlace+placeId:1818+universeId:13058"
                .to_string(),
        ];
        let translated = translate_protocol_args(&input);
        assert_eq!(
            translated,
            vec![
                "-task",
                "EditPlace",
                "-placeId",
                "1818",
                "-universeId",
                "13058"
            ]
        );
    }

    #[test]
    fn test_translate_studio_oauth_uri() {
        let input = vec!["roblox-studio://auth/callback?code=123".to_string()];
        let translated = translate_protocol_args(&input);
        assert_eq!(
            translated,
            vec!["-url", "roblox-studio://auth/callback?code=123"]
        );
    }
}

