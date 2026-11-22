mod config;
mod deployment;
mod desktop;
mod sober;
mod wine;

use clap::{Parser, Subcommand};
use config::AppConfig;
use deployment::{check_cdn_manifest, ensure_deployed, write_app_settings, write_fast_flags, BinaryType};
use std::fs;
use std::path::PathBuf;
use wine::WineRunner;

#[derive(Parser)]
#[command(
    name = "rbx-linux",
    version,
    about = "ตัวรันและจัดการ Roblox Windows (.exe) บน Linux ผ่าน Wine / Proton"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// ดาวน์โหลด/อัปเดต และเปิดใช้งาน Roblox Studio (WindowsStudio64) บน Linux
    Studio {
        /// อาร์กิวเมนต์หรือลิงก์ URI (เช่น roblox-studio:...)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// เปิดเล่นเกม Roblox ออนไลน์บน Linux (ใช้ Sober เป็นค่าเริ่มต้น หรือสลับไป Wine ได้ใน config)
    Player {
        /// อาร์กิวเมนต์หรือลิงก์ URI (เช่น roblox-player:...)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// รันไฟล์ Roblox .exe ที่กำหนดเอง (เช่น ไคลเอนต์เฉพาะ หรือ Studio แบบออฟไลน์) ใน Wine Prefix
    Run {
        /// พาธไปยังไฟล์ .exe ที่ต้องการรัน
        exe: PathBuf,
        /// อาร์กิวเมนต์เพิ่มเติมที่ส่งให้ไฟล์ .exe
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// ตั้งค่าเริ่มต้น: สร้าง Wine Prefix, เปิดใช้งาน DXVK และลงทะเบียน .desktop + ลิงก์เว็บ
    Setup,
    /// แสดงหรือตั้งค่าไฟล์ config.toml (เช่น เปลี่ยน Renderer D3D11/Vulkan/OpenGL, เลือก Proton หรือ player-backend)
    Config {
        /// ตั้งค่า Player Backend (sober หรือ wine)
        #[arg(long)]
        player_backend: Option<String>,
        /// ตั้งค่า Renderer (D3D11, Vulkan, OpenGL)
        #[arg(long)]
        renderer: Option<String>,
        /// ตั้งค่าพาธ Wine binary (เช่น wine หรือ /path/to/wine)
        #[arg(long)]
        wine: Option<String>,
        /// ตั้งค่าพาธ Proton directory (เช่น ~/.steam/steam/steamapps/common/Proton - Experimental)
        #[arg(long)]
        proton: Option<String>,
        /// เปิดหรือปิด DXVK (true/false)
        #[arg(long)]
        dxvk: Option<bool>,
    },
    /// แสดงสถานะการติดตั้ง พาธของ Wine Prefix และเวอร์ชันที่ติดตั้งไว้
    Status,
    /// บังคับปิดโปรเซส Roblox และ Wine ทั้งหมดที่ค้างอยู่ใน Prefix (wineserver -k)
    Kill,
    /// ล้างไฟล์แคช (.zip) หรือลบเวอร์ชันเก่าที่ไม่ได้ใช้งาน
    Clean {
        /// ลบ Wine Prefix และเวอร์ชันที่ดาวน์โหลดไว้ทั้งหมด
        #[arg(long)]
        all: bool,
    },
    /// เปิดหน้าต่างตั้งค่า Wine (winecfg) ของ Prefix นี้
    Winecfg,
    /// ทดสอบการเชื่อมต่อ Roblox CDN, ตรวจสอบรายการแพ็กเกจ (.zip) และการตั้งค่า Wine/FastFlags
    Check,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("\n[ข้อผิดพลาด / Error] {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let mut cfg = AppConfig::load()?;

    match cli.command {
        Some(Commands::Studio { args }) => {
            let exe_path = ensure_deployed(BinaryType::WindowsStudio64, &cfg)?;
            let runner = WineRunner::new(&cfg);
            runner.launch(&exe_path, &args)?;
        }
        Some(Commands::Player { args }) => {
            if cfg.player_backend.eq_ignore_ascii_case("wine") {
                eprintln!(
                    "[คำเตือน] คุณกำลังใช้ player_backend = \"wine\" ซึ่งระบบกันโกง Hyperion ของ Roblox บน Windows บล็อก Wine บน Linux จึงอาจไม่สามารถเข้าเล่นเซิร์ฟเวอร์ออนไลน์ได้ (แนะนำให้ใช้ `rbx-linux config --player-backend sober`)"
                );
                let exe_path = ensure_deployed(BinaryType::WindowsPlayer, &cfg)?;
                let runner = WineRunner::new(&cfg);
                runner.launch(&exe_path, &args)?;
            } else {
                sober::launch(&args, &cfg.merged_fflags(false))?;
            }
        }
        Some(Commands::Run { exe, args }) => {
            if !exe.exists() {
                return Err(format!("ไม่พบไฟล์ .exe ที่ระบุ: {:?}", exe));
            }
            if let Some(parent) = exe.parent() {
                let _ = write_app_settings(parent);
                let is_studio = exe
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_lowercase().contains("studio"))
                    .unwrap_or(false);
                let _ = write_fast_flags(parent, &cfg.merged_fflags(is_studio));
            }
            let runner = WineRunner::new(&cfg);
            runner.launch(&exe, &args)?;
        }
        Some(Commands::Setup) => {
            let runner = WineRunner::new(&cfg);
            runner.init_prefix()?;
            desktop::install_desktop_entries()?;
            if !cfg.player_backend.eq_ignore_ascii_case("wine") && !sober::sober_installed() {
                if let Err(e) = sober::install_sober() {
                    eprintln!("[คำเตือน] ข้ามการติดตั้ง Sober อัตโนมัติ: {}", e);
                }
            }
            println!("\n[สำเร็จ] ตั้งค่าระบบเสร็จสมบูรณ์! คุณสามารถรัน `rbx-linux player` (เล่นเกมออนไลน์ผ่าน Sober), `rbx-linux studio` หรือ `rbx-linux run <ไฟล์.exe>` ได้ทันที");
        }
        Some(Commands::Config {
            player_backend,
            renderer,
            wine,
            proton,
            dxvk,
        }) => {
            let mut changed = false;
            if let Some(pb) = player_backend {
                let norm = pb.trim().to_lowercase();
                if norm != "sober" && norm != "wine" {
                    return Err(format!(
                        "ค่า --player-backend ไม่ถูกต้อง: '{}' (รองรับเฉพาะ 'sober' หรือ 'wine')",
                        pb
                    ));
                }
                cfg.player_backend = norm;
                changed = true;
            }
            if let Some(r) = renderer {
                cfg.renderer = r;
                changed = true;
            }
            if let Some(w) = wine {
                cfg.wine_binary = w;
                changed = true;
            }
            if let Some(p) = proton {
                cfg.proton_dir = if p.is_empty() || p == "none" {
                    None
                } else {
                    Some(p)
                };
                changed = true;
            }
            if let Some(d) = dxvk {
                cfg.dxvk = d;
                changed = true;
            }

            if changed {
                cfg.save()?;
                println!("[Config] บันทึกการตั้งค่าใหม่ลงที่ {:?} แล้ว", AppConfig::config_file());
            }

            println!("=== การตั้งค่าปัจจุบัน ({:?}) ===", AppConfig::config_file());
            println!(
                "{}",
                toml::to_string_pretty(&cfg).unwrap_or_else(|_| "ไม่สามารถแสดงผลได้".to_string())
            );
        }
        Some(Commands::Status) => {
            print_status(&cfg);
        }
        Some(Commands::Kill) => {
            let runner = WineRunner::new(&cfg);
            runner.kill_prefix()?;
        }
        Some(Commands::Clean { all }) => {
            let cache = AppConfig::cache_dir();
            if cache.exists() {
                fs::remove_dir_all(&cache)
                    .map_err(|e| format!("ลบโฟลเดอร์แคชไม่สำเร็จ: {}", e))?;
                println!("[Clean] ล้างโฟลเดอร์แคช {:?} เรียบร้อยแล้ว", cache);
            }
            if all {
                let data = AppConfig::data_dir();
                if data.exists() {
                    fs::remove_dir_all(&data)
                        .map_err(|e| format!("ลบโฟลเดอร์ข้อมูลไม่สำเร็จ: {}", e))?;
                    println!("[Clean] ลบ Wine Prefix และเวอร์ชันทั้งหมดใน {:?} เรียบร้อยแล้ว", data);
                }
            }
        }
        Some(Commands::Winecfg) => {
            let runner = WineRunner::new(&cfg);
            let _ = runner
                .build_command("winecfg")
                .status()
                .map_err(|e| format!("เปิด winecfg ไม่สำเร็จ: {}", e))?;
        }
        Some(Commands::Check) => {
            println!("=== กำลังทดสอบระบบ rbx-linux (Self-Check & CDN Verification) ===");
            let (studio_ver, studio_pkgs) = check_cdn_manifest(BinaryType::WindowsStudio64, &cfg)?;
            println!(
                "[OK] WindowsStudio64 ล่าสุด: {} (พบ {} แพ็กเกจ .zip บน Roblox CDN)",
                studio_ver, studio_pkgs
            );
            let (player_ver, player_pkgs) = check_cdn_manifest(BinaryType::WindowsPlayer, &cfg)?;
            println!(
                "[OK] WindowsPlayer ล่าสุด  : {} (พบ {} แพ็กเกจ .zip บน Roblox CDN)",
                player_ver, player_pkgs
            );
            println!(
                "[OK] FastFlags (Studio)    : {:?}",
                cfg.merged_fflags(true)
            );
            let has_flatpak = sober::flatpak_available();
            let has_sober = has_flatpak && sober::sober_installed();
            println!(
                "[Info] Player Backend      : {} (Flatpak: {}, Sober: {})",
                cfg.player_backend,
                if has_flatpak { "พร้อมใช้งาน" } else { "ไม่พบ" },
                if has_sober { "ติดตั้งแล้ว" } else { "ไม่พบ" }
            );
            println!("[สำเร็จ] ระบบดาวน์โหลด CDN และการตั้งค่าพร้อมใช้งาน 100%");
        }
        None => {
            print_status(&cfg);
            println!("\nคำสั่งที่ใช้บ่อย:");
            println!("  rbx-linux setup               # ตั้งค่า Wine Prefix, DXVK, Sober และทางลัดบน Linux ครั้งแรก");
            println!("  rbx-linux player              # เล่นเกม Roblox ออนไลน์บน Linux ผ่าน Sober (ค่าเริ่มต้น)");
            println!("  rbx-linux studio              # ดาวน์โหลดและเปิด Roblox Studio (Windows .exe) บน Linux");
            println!("  rbx-linux run <path/to.exe>   # รันไฟล์ไคลเอนต์ Roblox Windows (.exe) ที่ต้องการใน Wine Prefix");
            println!("  rbx-linux config --renderer Vulkan  # เปลี่ยนโหมดกราฟิก (D3D11, Vulkan, OpenGL)");
            println!("  rbx-linux kill                # ปิดโปรเซส Roblox/Wine ที่ค้างอยู่ทั้งหมด");
        }
    }

    Ok(())
}

fn print_status(cfg: &AppConfig) {
    println!("=== สถานะ rbx-linux (Windows Roblox on Linux Wrapper) ===");
    println!("ไฟล์ตั้งค่า (Config) : {:?}", AppConfig::config_file());
    println!("โฟลเดอร์ Wine Prefix : {:?}", AppConfig::prefix_dir());
    println!("โฟลเดอร์เก็บเวอร์ชัน   : {:?}", AppConfig::versions_dir());
    println!("โฟลเดอร์แคชดาวน์โหลด : {:?}", AppConfig::cache_dir());
    println!("Player Backend     : {}", cfg.player_backend);
    println!("Wine Binary        : {}", cfg.wine_binary);
    println!(
        "Proton Directory   : {}",
        cfg.proton_dir.as_deref().unwrap_or("(ไม่ได้ใช้ - ใช้ Wine ปกติ)")
    );
    println!(
        "Renderer / DXVK    : {} (DXVK: {})",
        cfg.renderer,
        if cfg.dxvk { "เปิด" } else { "ปิด" }
    );

    let versions_dir = AppConfig::versions_dir();
    if let Ok(entries) = fs::read_dir(&versions_dir) {
        let installed: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        if installed.is_empty() {
            println!("เวอร์ชันที่ติดตั้งแล้ว  : (ยังไม่มี)");
        } else {
            println!("เวอร์ชันที่ติดตั้งแล้ว  : {}", installed.join(", "));
        }
    } else {
        println!("เวอร์ชันที่ติดตั้งแล้ว  : (ยังไม่มี)");
    }
}
