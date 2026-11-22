#!/usr/bin/env bash
set -euo pipefail

echo "====================================================="
echo "  ตัวติดตั้ง rbx-linux (Roblox Windows on Linux Wrapper)"
echo "====================================================="

# 1. ตรวจสอบว่ามี Wine ติดตั้งหรือยัง
if ! command -v wine >/dev/null 2>&1; then
    echo "[!] ยังไม่พบโปรแกรม Wine บนเครื่อง"
    if command -v apt >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Wine ผ่าน apt..."
        sudo dpkg --add-architecture i386 || true
        sudo apt update && sudo apt install -y wine64 wine32 winetricks
    elif command -v pacman >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Wine ผ่าน pacman..."
        sudo pacman -S --needed --noconfirm wine winetricks
    elif command -v dnf >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Wine ผ่าน dnf..."
        sudo dnf install -y wine winetricks
    else
        echo "กรุณาติดตั้งแพ็กเกจ wine และ winetricks ของ Linux Distro ของคุณก่อน"
    fi
fi

# 1.5 ตรวจสอบ Flatpak และติดตั้ง Sober สำหรับเล่นเกม Roblox ออนไลน์บน Linux
if ! command -v flatpak >/dev/null 2>&1; then
    echo "[!] ยังไม่พบโปรแกรม Flatpak บนเครื่อง (จำเป็นสำหรับ Sober)"
    if command -v apt >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Flatpak ผ่าน apt..."
        sudo apt update && sudo apt install -y flatpak
    elif command -v pacman >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Flatpak ผ่าน pacman..."
        sudo pacman -S --needed --noconfirm flatpak
    elif command -v dnf >/dev/null 2>&1; then
        echo " -> กำลังติดตั้ง Flatpak ผ่าน dnf..."
        sudo dnf install -y flatpak
    else
        echo "กรุณาติดตั้งแพ็กเกจ flatpak ของ Linux Distro ของคุณก่อน"
    fi
fi

if command -v flatpak >/dev/null 2>&1; then
    echo "[*] กำลังเพิ่ม Flathub และติดตั้ง Sober (org.vinegarhq.Sober)..."
    flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak install --user -y flathub org.vinegarhq.Sober || echo "[!] ข้ามการติดตั้ง Sober อัตโนมัติ (สามารถรัน rbx-linux player เพื่อลองติดตั้งใหม่ภายหลังได้)"
fi

# 2. ตรวจสอบ Rust / Cargo สำหรับคอมไพล์ Binary
if ! command -v cargo >/dev/null 2>&1; then
    echo "[*] ไม่พบ Cargo กำลังติดตั้ง Rust Toolchain อัตโนมัติ..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

# 3. คอมไพล์ rbx-linux โหมด Release (Binary เดียว ประสิทธิภาพสูง)
echo "[*] กำลังคอมไพล์ rbx-linux (Release Mode)..."
cargo build --release

# 4. ติดตั้ง Binary ไปยัง ~/.local/bin
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp -f target/release/rbx-linux "$INSTALL_DIR/rbx-linux"
chmod +x "$INSTALL_DIR/rbx-linux"

echo "[*] ติดตั้งไบนารีไปที่: $INSTALL_DIR/rbx-linux"

# 5. รันตั้งค่า Wine Prefix และ Desktop Entries
"$INSTALL_DIR/rbx-linux" setup

echo ""
echo "====================================================="
echo "  ติดตั้งเสร็จสมบูรณ์! วิธีใช้งาน:"
echo "  - เล่น Roblox ออนไลน์ : rbx-linux player (ผ่าน Sober)"
echo "  - เปิด Roblox Studio  : rbx-linux studio (ผ่าน Wine)"
echo "  - รันไฟล์ .exe เฉพาะ  : rbx-linux run /path/to/Roblox.exe"
echo "  - ดูสถานะการตั้งค่า    : rbx-linux status"
echo "====================================================="
