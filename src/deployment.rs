use crate::config::AppConfig;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const CDN_BASE: &str = "https://setup.rbxcdn.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    WindowsStudio64,
    WindowsPlayer,
}

impl BinaryType {
    pub fn api_name(&self) -> &'static str {
        match self {
            BinaryType::WindowsStudio64 => "WindowsStudio64",
            BinaryType::WindowsPlayer => "WindowsPlayer",
        }
    }

    pub fn executable_name(&self) -> &'static str {
        match self {
            BinaryType::WindowsStudio64 => "RobloxStudioBeta.exe",
            BinaryType::WindowsPlayer => "RobloxPlayerBeta.exe",
        }
    }

    pub fn is_studio(&self) -> bool {
        matches!(self, BinaryType::WindowsStudio64)
    }
}

#[derive(Debug, Deserialize)]
struct ClientVersionResponse {
    #[serde(rename = "clientVersionUpload")]
    client_version_upload: String,
}

/// Resolve the target Roblox version hash (either forced from config or latest from CDN)
pub fn resolve_version(bin_type: BinaryType, cfg: &AppConfig) -> Result<String, String> {
    if let Some(forced) = match bin_type {
        BinaryType::WindowsStudio64 => &cfg.forced_studio_version,
        BinaryType::WindowsPlayer => &cfg.forced_player_version,
    } {
        if !forced.trim().is_empty() {
            return Ok(forced.trim().to_string());
        }
    }

    let channel = cfg.channel.trim();
    let url = if channel.is_empty() || channel.eq_ignore_ascii_case("LIVE") {
        format!(
            "https://clientsettings.roblox.com/v2/client-version/{}",
            bin_type.api_name()
        )
    } else {
        format!(
            "https://clientsettings.roblox.com/v2/client-version/{}/channel/{}",
            bin_type.api_name(),
            channel
        )
    };

    println!("[CDN] กำลังตรวจสอบเวอร์ชันล่าสุดของ {}...", bin_type.api_name());
    let resp: ClientVersionResponse = ureq::get(&url)
        .call()
        .map_err(|e| format!("ไม่สามารถดึงข้อมูลเวอร์ชันจาก {}: {}", url, e))?
        .into_json()
        .map_err(|e| format!("แปลง JSON ข้อมูลเวอร์ชันไม่สำเร็จ: {}", e))?;

    Ok(resp.client_version_upload)
}

/// Maps a Roblox package zip name to its target extraction directory relative to the version root
fn package_target_subdir(bin_type: BinaryType, pkg_name: &str) -> Option<&'static str> {
    match bin_type {
        BinaryType::WindowsStudio64 => match pkg_name {
            "RobloxStudio.zip"
            | "redist.zip"
            | "Libraries.zip"
            | "LibrariesQt5.zip"
            | "WebView2.zip" => Some(""),
            "ApplicationConfig.zip" => Some("ApplicationConfig"),
            "content-avatar.zip" => Some("content/avatar"),
            "content-configs.zip" => Some("content/configs"),
            "content-fonts.zip" => Some("content/fonts"),
            "content-models.zip" => Some("content/models"),
            "content-LuaPackages.zip" => Some("ExtraContent/LuaPackages"),
            "content-luapackages.zip" => Some("ExtraContent/LuaPackages"),
            "content-platform-dictionaries.zip" => Some("PlatformContent/pc/shared_compression_dictionaries"),
            "content-platform-fonts.zip" => Some("PlatformContent/pc/fonts"),
            "content-qt_translations.zip" => Some("content/qt_translations"),
            "content-sky.zip" => Some("content/sky"),
            "content-sounds.zip" => Some("content/sounds"),
            "content-studio_svg_textures.zip" => Some("content/studio_svg_textures"),
            "content-terrain.zip" => Some("PlatformContent/pc/terrain"),
            "content-textures2.zip" => Some("content/textures"),
            "content-textures3.zip" => Some("PlatformContent/pc/textures"),
            "content-translations.zip" => Some("ExtraContent/translations"),
            "extracontent-places.zip" => Some("ExtraContent/places"),
            "extracontent-luapackages.zip" => Some("ExtraContent/LuaPackages"),
            "extracontent-models.zip" => Some("ExtraContent/models"),
            "extracontent-textures.zip" => Some("ExtraContent/textures"),
            "extracontent-translations.zip" => Some("ExtraContent/translations"),
            "BuiltInPlugins.zip" => Some("BuiltInPlugins"),
            "BuiltInStandalonePlugins.zip" => Some("BuiltInStandalonePlugins"),
            "Plugins.zip" => Some("Plugins"),
            "Qml.zip" => Some("Qml"),
            "StudioFonts.zip" => Some("StudioFonts"),
            "shaders.zip" => Some("shaders"),
            "ssl.zip" => Some("ssl"),
            "WebView2RuntimeInstaller.zip" => Some("WebView2RuntimeInstaller"),
            _ => Some(""),
        },
        BinaryType::WindowsPlayer => match pkg_name {
            "RobloxApp.zip" | "redist.zip" | "WebView2.zip" => Some(""),
            "shaders.zip" => Some("shaders"),
            "ssl.zip" => Some("ssl"),
            "WebView2RuntimeInstaller.zip" => Some("WebView2RuntimeInstaller"),
            "content-avatar.zip" => Some("content/avatar"),
            "content-configs.zip" => Some("content/configs"),
            "content-fonts.zip" => Some("content/fonts"),
            "content-sky.zip" => Some("content/sky"),
            "content-sounds.zip" => Some("content/sounds"),
            "content-textures2.zip" => Some("content/textures"),
            "content-models.zip" => Some("content/models"),
            "content-platform-dictionaries.zip" => Some("PlatformContent/pc/shared_compression_dictionaries"),
            "content-platform-fonts.zip" => Some("PlatformContent/pc/fonts"),
            "content-terrain.zip" => Some("PlatformContent/pc/terrain"),
            "content-textures3.zip" => Some("PlatformContent/pc/textures"),
            "extracontent-luapackages.zip" => Some("ExtraContent/LuaPackages"),
            "extracontent-translations.zip" => Some("ExtraContent/translations"),
            "extracontent-models.zip" => Some("ExtraContent/models"),
            "extracontent-textures.zip" => Some("ExtraContent/textures"),
            "extracontent-places.zip" => Some("ExtraContent/places"),
            _ => Some(""),
        },
    }
}

/// Ensure the specified version of WindowsStudio64 or WindowsPlayer is downloaded & extracted
pub fn ensure_deployed(bin_type: BinaryType, cfg: &AppConfig) -> Result<PathBuf, String> {
    let version = resolve_version(bin_type, cfg)?;
    let version_dir = AppConfig::versions_dir().join(&version);
    let exe_path = version_dir.join(bin_type.executable_name());
    let marker_path = version_dir.join(".deployed_ok");

    if exe_path.exists() && marker_path.exists() {
        println!(
            "[Ready] พบ {} ({}) ติดตั้งเรียบร้อยแล้ว",
            bin_type.api_name(),
            version
        );
        write_app_settings(&version_dir)?;
        write_fast_flags(&version_dir, &cfg.merged_fflags(bin_type.is_studio()))?;
        return Ok(exe_path);
    }

    println!(
        "[Deploy] กำลังดาวน์โหลดและติดตั้ง {} เวอร์ชัน {}...",
        bin_type.api_name(),
        version
    );
    fs::create_dir_all(&version_dir)
        .map_err(|e| format!("สร้างโฟลเดอร์เวอร์ชันไม่สำเร็จ: {}", e))?;

    let cache_dir = AppConfig::cache_dir().join(&version);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("สร้างโฟลเดอร์แคชไม่สำเร็จ: {}", e))?;

    // Fetch rbxPkgManifest.txt
    let manifest_url = format!("{}/{}-rbxPkgManifest.txt", CDN_BASE, version);
    let manifest_text = ureq::get(&manifest_url)
        .call()
        .map_err(|e| format!("ดาวน์โหลด manifest จาก {} ไม่สำเร็จ: {}", manifest_url, e))?
        .into_string()
        .map_err(|e| format!("อ่านข้อมูล manifest ไม่สำเร็จ: {}", e))?;

    let packages: Vec<&str> = manifest_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.ends_with(".zip"))
        .collect();

    if packages.is_empty() {
        return Err("ไม่พบรายการไฟล์ .zip ใน rbxPkgManifest.txt".to_string());
    }

    for (idx, pkg) in packages.iter().enumerate() {
        // Skip Launcher executable or installer wrapper if not needed
        if *pkg == "RobloxPlayerLauncher.exe" || *pkg == "WindowsStudio64.exe" {
            continue;
        }
        let Some(subdir) = package_target_subdir(bin_type, pkg) else {
            continue;
        };

        let pkg_url = format!("{}/{}-{}", CDN_BASE, version, pkg);
        let zip_cache_path = cache_dir.join(pkg);

        println!(
            "  [{}/{}] ดาวน์โหลด {}...",
            idx + 1,
            packages.len(),
            pkg
        );
        download_file(&pkg_url, &zip_cache_path)?;

        let target_dir = if subdir.is_empty() {
            version_dir.clone()
        } else {
            version_dir.join(subdir)
        };

        extract_zip(&zip_cache_path, &target_dir)?;
    }

    write_app_settings(&version_dir)?;
    write_fast_flags(&version_dir, &cfg.merged_fflags(bin_type.is_studio()))?;

    fs::write(&marker_path, &version)
        .map_err(|e| format!("ไม่สามารถสร้างไฟล์ยืนยันการติดตั้ง: {}", e))?;

    println!("[Deploy] ติดตั้ง {} ({}) เสร็จสมบูรณ์!", bin_type.api_name(), version);
    Ok(exe_path)
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(());
    }
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("ดาวน์โหลด {} ล้มเหลว: {}", url, e))?;

    let mut reader = resp.into_reader();
    let mut file = File::create(dest)
        .map_err(|e| format!("สร้างไฟล์ {:?} ไม่สำเร็จ: {}", dest, e))?;
    io::copy(&mut reader, &mut file)
        .map_err(|e| format!("บันทึกข้อมูลลงไฟล์ {:?} ไม่สำเร็จ: {}", dest, e))?;
    Ok(())
}

fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("สร้างโฟลเดอร์ {:?} ไม่สำเร็จ: {}", target_dir, e))?;

    let file = File::open(zip_path)
        .map_err(|e| format!("เปิดไฟล์ zip {:?} ไม่สำเร็จ: {}", zip_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("อ่านโครงสร้าง zip {:?} ไม่สำเร็จ: {}", zip_path, e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("อ่านรายการใน zip ไม่สำเร็จ: {}", e))?;

        let raw_name = entry.name().replace('\\', "/");
        let clean_rel = Path::new(&raw_name);

        // Prevent path traversal
        if clean_rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            continue;
        }

        let out_path = target_dir.join(clean_rel);

        if entry.name().ends_with('/') || entry.is_dir() {
            fs::create_dir_all(&out_path).ok();
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut outfile = File::create(&out_path)
                .map_err(|e| format!("สร้างไฟล์ {:?} ไม่สำเร็จ: {}", out_path, e))?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("แตกไฟล์ {:?} ไม่สำเร็จ: {}", out_path, e))?;
            outfile
                .write_all(&buf)
                .map_err(|e| format!("เขียนไฟล์ {:?} ไม่สำเร็จ: {}", out_path, e))?;
        }
    }
    Ok(())
}

pub fn write_app_settings(version_dir: &Path) -> Result<(), String> {
    let xml = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<Settings>\n",
        "\t<ContentFolder>content</ContentFolder>\n",
        "\t<BaseUrl>http://www.roblox.com</BaseUrl>\n",
        "</Settings>\n"
    );
    fs::write(version_dir.join("AppSettings.xml"), xml)
        .map_err(|e| format!("เขียนไฟล์ AppSettings.xml ไม่สำเร็จ: {}", e))
}

pub fn write_fast_flags(
    version_dir: &Path,
    fflags: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    let client_settings_dir = version_dir.join("ClientSettings");
    fs::create_dir_all(&client_settings_dir)
        .map_err(|e| format!("สร้างโฟลเดอร์ ClientSettings ไม่สำเร็จ: {}", e))?;

    let json_str = serde_json::to_string_pretty(fflags)
        .map_err(|e| format!("แปลง FastFlags เป็น JSON ไม่สำเร็จ: {}", e))?;

    fs::write(client_settings_dir.join("ClientAppSettings.json"), json_str)
        .map_err(|e| format!("เขียนไฟล์ ClientAppSettings.json ไม่สำเร็จ: {}", e))
}

/// Self-test helper: checks live Roblox CDN version & package manifest for a binary type
pub fn check_cdn_manifest(bin_type: BinaryType, cfg: &AppConfig) -> Result<(String, usize), String> {
    let version = resolve_version(bin_type, cfg)?;
    let manifest_url = format!("{}/{}-rbxPkgManifest.txt", CDN_BASE, version);
    let manifest_text = ureq::get(&manifest_url)
        .call()
        .map_err(|e| format!("ดึง manifest จาก {} ไม่สำเร็จ: {}", manifest_url, e))?
        .into_string()
        .map_err(|e| format!("อ่านข้อมูล manifest ไม่สำเร็จ: {}", e))?;

    let zip_count = manifest_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.ends_with(".zip"))
        .count();

    if zip_count == 0 {
        return Err(format!("ไม่พบแพ็กเกจ .zip ใน manifest ของ {}", version));
    }

    Ok((version, zip_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_mapping_studio_and_player() {
        assert_eq!(
            package_target_subdir(BinaryType::WindowsStudio64, "RobloxStudio.zip"),
            Some("")
        );
        assert_eq!(
            package_target_subdir(BinaryType::WindowsStudio64, "BuiltInPlugins.zip"),
            Some("BuiltInPlugins")
        );
        assert_eq!(
            package_target_subdir(BinaryType::WindowsPlayer, "RobloxApp.zip"),
            Some("")
        );
        assert_eq!(
            package_target_subdir(BinaryType::WindowsPlayer, "shaders.zip"),
            Some("shaders")
        );
    }

    #[test]
    fn test_write_settings_and_fflags() {
        let temp_dir = std::env::temp_dir().join("rbx_linux_test_settings");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        write_app_settings(&temp_dir).unwrap();
        assert!(temp_dir.join("AppSettings.xml").exists());

        let cfg = AppConfig::default();
        let flags = cfg.merged_fflags(true);
        write_fast_flags(&temp_dir, &flags).unwrap();

        let fflag_file = temp_dir.join("ClientSettings").join("ClientAppSettings.json");
        assert!(fflag_file.exists());
        let content = fs::read_to_string(fflag_file).unwrap();
        assert!(content.contains("FFlagDebugGraphicsPreferD3D11"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_live_cdn_version_and_manifest() {
        let cfg = AppConfig::default();
        let (studio_ver, studio_zips) =
            check_cdn_manifest(BinaryType::WindowsStudio64, &cfg).expect("Studio CDN check failed");
        assert!(studio_ver.starts_with("version-"));
        assert!(studio_zips > 10);

        let (player_ver, player_zips) =
            check_cdn_manifest(BinaryType::WindowsPlayer, &cfg).expect("Player CDN check failed");
        assert!(player_ver.starts_with("version-"));
        assert!(player_zips > 5);
    }
}

