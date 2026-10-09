//! App icons. Windows draws the icon for any path or shell:AppsFolder item via
//! IShellItemImageFactory; we turn that bitmap into a PNG data URL, then cache it
//! in memory and on disk so each icon is extracted only once.
use base64::Engine;
use parking_lot::Mutex;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::OnceLock,
};

const SIZE: i32 = 64;

fn memory() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn disk_path(cache_dir: &Path, target: &str) -> PathBuf {
    let mut h = DefaultHasher::new();
    target.hash(&mut h);
    cache_dir.join(format!("{:016x}.png", h.finish()))
}

fn to_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// Icon for an app path / shell:AppsFolder target, as a data URL. Blocking: call off the UI thread.
pub fn get(target: &str, cache_dir: &Path) -> Option<String> {
    if let Some(hit) = memory().lock().get(target) {
        return hit.clone();
    }
    let file = disk_path(cache_dir, target);
    let result = match fs::read(&file) {
        Ok(bytes) => Some(to_data_url(&bytes)),
        Err(_) => extract_png(target).map(|png| {
            let _ = fs::create_dir_all(cache_dir);
            let _ = fs::write(&file, &png);
            to_data_url(&png)
        }),
    };
    memory().lock().insert(target.to_string(), result.clone());
    result
}

fn encode_png(rgba: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(out)
}

#[cfg(windows)]
fn extract_png(target: &str) -> Option<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Win32::{
        Foundation::SIZE,
        Graphics::Gdi::*,
        System::Com::{CoInitializeEx, IBindCtx, COINIT_APARTMENTTHREADED},
        UI::Shell::*,
    };

    unsafe {
        // Harmless if this thread is already initialised.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(target), None::<&IBindCtx>).ok()?;
        let hbmp = factory
            .GetImage(SIZE { cx: SIZE_PX, cy: SIZE_PX }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;

        let mut bmp = BITMAP::default();
        GetObjectW(
            HGDIOBJ(hbmp.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut core::ffi::c_void),
        );
        let (w, h) = (bmp.bmWidth, bmp.bmHeight);
        if w <= 0 || h <= 0 {
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            return None;
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // negative = top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let hdc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            hdc,
            hbmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        if lines == 0 {
            return None;
        }

        // BGRA (premultiplied) -> straight RGBA. Old icons may have no alpha at all.
        let has_alpha = buf.chunks_exact(4).any(|p| p[3] != 0);
        for p in buf.chunks_exact_mut(4) {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            } else if p[3] > 0 && p[3] < 255 {
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        encode_png(&buf, w as u32, h as u32)
    }
}

#[cfg(windows)]
const SIZE_PX: i32 = SIZE;

#[cfg(not(windows))]
fn extract_png(_target: &str) -> Option<Vec<u8>> {
    None
}
