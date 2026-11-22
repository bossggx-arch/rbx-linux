# rbx-linux (Roblox Windows on Linux Launcher & Wine/Proton Wrapper)

ตัวรันและจัดการ **Roblox เวอร์ชัน Windows (`.exe`) บนระบบปฏิบัติการ Linux** เขียนด้วยภาษา **Rust** คอมไพล์เป็นไฟล์ Binary เดียวขนาดเล็ก ประสิทธิภาพสูง และใช้งานง่ายที่สุด

## เล่น Roblox ออนไลน์บน Linux

เนื่องจากตัวเกม **Roblox Player เวอร์ชัน Windows** มีระบบป้องกัน **Hyperion** ซึ่งบล็อกการทำงานผ่าน Wine/Proton บน Linux ดังนั้นคำสั่ง `rbx-linux player` จึงตั้งค่าเริ่มต้นให้ใช้ **[Sober](https://sober.vinegarhq.org/)** (ตัวรัน Roblox เวอร์ชัน Android บน Linux แบบ Native ผ่าน Flatpak พัฒนาโดยทีม VinegarHQ) เพื่อให้สามารถเข้าเล่นเกม Roblox ออนไลน์ได้จริงและลื่นไหล

ส่วน **Roblox Studio (`rbx-linux studio`)** และการรันไฟล์ `.exe` เฉพาะ (`rbx-linux run`) ยังคงทำงานผ่าน **Wine / Proton** ตามเดิม

---

## ฟีเจอร์หลัก
- **เล่น Roblox ออนไลน์ได้จริงบน Linux (`rbx-linux player`)**: เรียกใช้งาน Sober (`org.vinegarhq.Sober`) ผ่าน Flatpak โดยอัตโนมัติ (ติดตั้งให้อัตโนมัติหากยังไม่มี พร้อมซิงก์ FastFlags และรองรับลิงก์ `roblox://`) และสามารถสลับกลับไปใช้ Wine ได้ผ่าน `rbx-linux config --player-backend wine`
- **ดาวน์โหลดและอัปเดตจาก Roblox CDN อัตโนมัติ**: ดึงไฟล์แพ็กเกจล่าสุดจาก `clientsettings.roblox.com` และ `setup.rbxcdn.com` แตกไฟล์เข้าโฟลเดอร์ย่อยที่ถูกต้อง พร้อมสร้าง `AppSettings.xml` ให้ทันที
- **จัดการ Wine / Proton Prefix อัตโนมัติ**: สร้างและตั้งค่า 64-bit Wine Prefix (`~/.local/share/rbx-linux/pfx`) เป็น Windows 10 พร้อมเปิดใช้งาน `WINEESYNC`, `WINEFSYNC` และ **DXVK** (แปลง Direct3D 11 เป็น Vulkan เพื่อเฟรมเรตที่ลื่นไหลบน Linux)
- **รันไฟล์ `.exe` ที่กำหนดเองได้ (`rbx-linux run`)**: หากมีไฟล์ไคลเอนต์ Roblox Windows (`.exe`) เฉพาะหรือตัวเกมออฟไลน์ สามารถสั่งรันผ่านสภาพแวดล้อม Wine/Proton ที่ตั้งค่า FastFlags และ DXVK ไว้แล้วได้ทันที
- **จัดการ FastFlags (`ClientAppSettings.json`)**: สลับโหมดกราฟิก (`D3D11`, `Vulkan`, `OpenGL`) หรือเพิ่ม FastFlags เองได้ผ่านไฟล์ `config.toml`
- **เชื่อมต่อกับเบราว์เซอร์บน Linux**: สร้างไฟล์ `.desktop` และลงทะเบียน URI Scheme (`roblox-studio://`, `roblox-player://`) ให้คลิกเปิดจากหน้าเว็บได้ทันที

---

## วิธีติดตั้งและใช้งานบน Linux (ง่ายที่สุดในคำสั่งเดียว)

คัดลอกโฟลเดอร์โปรเจกต์นี้ไปยังเครื่อง Linux ของคุณ แล้วรันสคริปต์:

```bash
chmod +x install.sh
./install.sh
```

หรือหากต้องการคอมไพล์ด้วยตนเอง:

```bash
cargo build --release
./target/release/rbx-linux setup
```

---

## คำสั่งการใช้งาน

| คำสั่ง | คำอธิบาย |
| :--- | :--- |
| `rbx-linux setup` | สร้าง Wine Prefix (Windows 10), ตั้งค่า DXVK, ติดตั้ง Sober (ถ้าใช้ backend sober) และสร้างไอคอน `.desktop` |
| `rbx-linux player` | เปิดเล่นเกม **Roblox ออนไลน์บน Linux** ผ่าน **Sober** (ค่าเริ่มต้น) หรือรัน `WindowsPlayer` ผ่าน Wine หากตั้งค่าไว้ |
| `rbx-linux studio` | ดาวน์โหลด/อัปเดต และเปิด **Roblox Studio (`RobloxStudioBeta.exe`)** ผ่าน Wine/Proton |
| `rbx-linux run <ไฟล์.exe>` | รันไฟล์ Roblox Windows `.exe` ที่คุณระบุภายใน Wine/Proton Prefix ที่ปรับแต่งแล้ว |
| `rbx-linux config --player-backend sober\|wine` | เลือก backend สำหรับคำสั่ง `player` (`sober` เพื่อเล่นออนไลน์ หรือ `wine`) |
| `rbx-linux config --renderer Vulkan` | เปลี่ยน Renderer กราฟิก (`D3D11`, `Vulkan`, `OpenGL`) |
| `rbx-linux config --proton "/path/to/Proton"` | เปลี่ยนไปใช้ Proton แทน Wine ปกติ |
| `rbx-linux check` | ตรวจสอบการเชื่อมต่อ Roblox CDN, FastFlags และสถานะ Flatpak / Sober |
| `rbx-linux winecfg` | เปิดหน้าต่างตั้งค่า `winecfg` ของ Prefix นี้ |
| `rbx-linux kill` | บังคับปิดโปรเซส Roblox และ Wine ที่ค้างอยู่ทั้งหมด (`wineserver -k`) |
| `rbx-linux clean` | ล้างไฟล์แคช `.zip` ที่ดาวน์โหลดมา (เติม `--all` เพื่อล้าง Wine Prefix และติดตั้งใหม่) |

---

## การปรับแต่งเพิ่มเติม (`~/.config/rbx-linux/config.toml`)

ไฟล์ตั้งค่าจะถูกสร้างให้อัตโนมัติเมื่อรันครั้งแรก ตัวอย่างโครงสร้าง:

```toml
player_backend = "sober"
wine_binary = "wine"
renderer = "D3D11"
dxvk = true
esync = true
fsync = true
channel = "LIVE"

[env]
# เพิ่มตัวแปร Environment ให้ Wine ได้ตามต้องการ เช่น:
# DXVK_HUD = "fps"

[fflags_studio]
FFlagDebugGraphicsPreferD3D11 = true

[fflags_player]
FFlagDebugGraphicsPreferD3D11 = true
```
