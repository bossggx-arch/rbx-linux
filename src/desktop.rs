use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn install_desktop_entries() -> Result<(), String> {
    let exe_path = env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("rbx-linux"))
        .to_string_lossy()
        .to_string();

    let apps_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("applications");

    fs::create_dir_all(&apps_dir)
        .map_err(|e| format!("สร้างโฟลเดอร์ applications ไม่สำเร็จ: {}", e))?;

    let studio_desktop = format!(
        concat!(
            "[Desktop Entry]\n",
            "Name=Roblox Studio (rbx-linux)\n",
            "Comment=Windows Roblox Studio running on Linux via Wine/Proton\n",
            "Exec=\"{exe}\" studio %u\n",
            "Terminal=false\n",
            "Type=Application\n",
            "Categories=Development;IDE;Game;\n",
            "MimeType=x-scheme-handler/roblox-studio;x-scheme-handler/roblox-studio-auth;application/x-roblox-rbxl;application/x-roblox-rbxlx;\n",
            "StartupWMClass=RobloxStudioBeta.exe\n"
        ),
        exe = exe_path
    );

    let player_desktop = format!(
        concat!(
            "[Desktop Entry]\n",
            "Name=Roblox Player (rbx-linux Wine Wrapper)\n",
            "Comment=Windows Roblox Client wrapper on Linux via Wine/Proton\n",
            "Exec=\"{exe}\" player %u\n",
            "Terminal=false\n",
            "Type=Application\n",
            "Categories=Game;\n",
            "MimeType=x-scheme-handler/roblox-player;x-scheme-handler/roblox;\n",
            "StartupWMClass=RobloxPlayerBeta.exe\n"
        ),
        exe = exe_path
    );

    let studio_file = apps_dir.join("rbx-linux-studio.desktop");
    let player_file = apps_dir.join("rbx-linux-player.desktop");

    fs::write(&studio_file, studio_desktop)
        .map_err(|e| format!("เขียนไฟล์ {:?} ไม่สำเร็จ: {}", studio_file, e))?;
    fs::write(&player_file, player_desktop)
        .map_err(|e| format!("เขียนไฟล์ {:?} ไม่สำเร็จ: {}", player_file, e))?;

    println!("[Desktop] สร้างทางลัดแอปพลิเคชันแล้ว:");
    println!("  - {:?}", studio_file);
    println!("  - {:?}", player_file);

    // Update desktop database & register MIME handlers on Linux if xdg-mime is present
    let _ = Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();

    for scheme in ["x-scheme-handler/roblox-studio", "x-scheme-handler/roblox-studio-auth"] {
        let _ = Command::new("xdg-mime")
            .args(["default", "rbx-linux-studio.desktop", scheme])
            .status();
    }

    for scheme in ["x-scheme-handler/roblox-player", "x-scheme-handler/roblox"] {
        let _ = Command::new("xdg-mime")
            .args(["default", "rbx-linux-player.desktop", scheme])
            .status();
    }

    println!("[Desktop] ลงทะเบียน URI Handlers (roblox-studio:// และ roblox-player://) เรียบร้อยแล้ว");
    Ok(())
}
