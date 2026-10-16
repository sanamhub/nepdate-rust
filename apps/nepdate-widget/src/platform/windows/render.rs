//! Direct2D + DirectWrite into a premultiplied BGRA buffer (ADR-0016 point 4). No window: the
//! caller passes the two lines, the DPI scale and the ink colour, and gets back the pixels and
//! their size, so the rendering is testable without a display. The bundled Noto fonts are loaded
//! from memory into a private collection, so there is no system-font dependency.
// W1B-03 wires this into `main.rs`; until then the binary does not call it. Remove this allow.
#![allow(dead_code)]
// FFI conventions the renderer cannot avoid: `&x` where `*const T` is expected, and the text
// dimensions, font sizes and DIB metrics, which are all a few hundred pixels or bytes at most.
#![allow(
    clippy::borrow_as_ptr,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use anyhow::{Context, bail};
use once_cell::unsync::OnceCell;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_PROPERTY, DWRITE_FONT_PROPERTY_ID_FAMILY_NAME,
    DWRITE_FONT_SIMULATIONS_NONE, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_TEXT_METRICS, DWRITE_UNICODE_RANGE, DWRITE_WORD_WRAPPING_NO_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteFactory3, IDWriteFactory5, IDWriteFontCollection,
    IDWriteFontFaceReference, IDWriteFontFallback, IDWriteFontFallbackBuilder, IDWriteFontSet,
    IDWriteFontSetBuilder, IDWriteInMemoryFontFileLoader, IDWriteTextFormat, IDWriteTextFormat1,
    IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, RGBQUAD, ReleaseDC, SelectObject,
};
use windows::core::Interface as _;
use windows::core::w;

use crate::app::taskbar::TaskbarLines;

/// The bundled fonts, embedded at compile time (ADR-0010, ADR-0016 point 4).
const NOTO_SANS: &[u8] = include_bytes!("../../../ui/fonts/NotoSans-Variable.ttf");
const NOTO_DEVANAGARI: &[u8] = include_bytes!("../../../ui/fonts/NotoSansDevanagari-Variable.ttf");

/// widget-design.md §5: line 1 regular, line 2 semibold, 12 px at 100 % scale, 2 px line gap.
const FONT_SIZE: f32 = 12.0;
const LINE_GAP: f32 = 2.0;

/// A rendered buffer: premultiplied BGRA, top-down, one byte per channel.
pub struct Rendered {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Renders `lines` at `scale` (dpi / 96) in `ink` (premultiplied RGBA). Every pixel gets at least
/// alpha 1, so the whole rectangle takes clicks (ADR-0016 point 4). The DirectWrite and Direct2D
/// factories, the font loader and the font collection are built once per thread (W1B-05 item 2).
pub fn render(lines: &TaskbarLines, scale: f32, ink: [u8; 4]) -> anyhow::Result<Rendered> {
    FONTS.with(|cell| {
        let fonts = cell.get_or_try_init(init_fonts)?;
        render_with(fonts, lines, scale, ink)
    })
}

/// The shared DirectWrite and Direct2D state for this thread. The controller thread is the only
/// render caller; the `--taskbar-snapshot` tool reuses the same cache.
struct Fonts {
    dw: IDWriteFactory5,
    d2d: ID2D1Factory,
    collection: IDWriteFontCollection,
    fallback: IDWriteFontFallback,
}

thread_local! {
    static FONTS: OnceCell<Fonts> = const { OnceCell::new() };
}

/// Builds the DirectWrite factory, the Direct2D factory, the in-memory font loader and the font
/// collection once for this thread. The loader is never unregistered, so this must run once, not
/// once per render.
fn init_fonts() -> anyhow::Result<Fonts> {
    // SAFETY: DirectWrite is a process-wide COM API; a shared factory is the documented default.
    let dw: IDWriteFactory5 = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
    // SAFETY: a single-threaded D2D factory is used on this thread only, as Direct2D documents.
    let d2d: ID2D1Factory = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
    let collection = build_collection(&dw)?;
    let fallback = devanagari_fallback(&dw, &collection)?;
    Ok(Fonts {
        dw,
        d2d,
        collection,
        fallback,
    })
}

/// The render body, using the cached factories, collection and fallback.
fn render_with(
    fonts: &Fonts,
    lines: &TaskbarLines,
    scale: f32,
    ink: [u8; 4],
) -> anyhow::Result<Rendered> {
    let dw = &fonts.dw;
    let collection = &fonts.collection;
    let fallback = &fonts.fallback;

    let format1 = text_format(dw, collection, DWRITE_FONT_WEIGHT_REGULAR, scale, fallback)?;
    let format2 = text_format(
        dw,
        collection,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        scale,
        fallback,
    )?;

    let layout1 = layout(dw, &lines.line1, &format1)?;
    let layout2 = layout(dw, &lines.line2, &format2)?;
    let (w1, h1) = metrics(&layout1)?;
    let (w2, h2) = metrics(&layout2)?;

    let width = w1.max(w2).ceil().max(1.0) as u32;
    let height = (h1 + LINE_GAP * scale + h2).ceil().max(1.0) as u32;

    let dib = Dib::new(width, height)?;
    let d2d = &fonts.d2d;
    let properties = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };
    // SAFETY: `properties` is a valid render-target description.
    let target: ID2D1DCRenderTarget = unsafe { d2d.CreateDCRenderTarget(&properties)? };
    let bounds = RECT {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    // SAFETY: the DIB's DC is valid for the render target's lifetime; `bounds` is the DIB's size.
    unsafe { target.BindDC(dib.hdc, &bounds)? };
    let colour = D2D1_COLOR_F {
        r: f32::from(ink[0]) / 255.0,
        g: f32::from(ink[1]) / 255.0,
        b: f32::from(ink[2]) / 255.0,
        a: f32::from(ink[3]) / 255.0,
    };
    // SAFETY: the brush is used only while `colour` and the target are alive.
    let brush = unsafe { target.CreateSolidColorBrush(&colour, None)? };

    // SAFETY: the layouts and brush outlive the draw; the origins stay inside the DIB.
    unsafe {
        target.BeginDraw();
        target.Clear(None);
        target.DrawTextLayout(
            windows_numerics::Vector2 { X: 0.0, Y: 0.0 },
            &layout1,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        target.DrawTextLayout(
            windows_numerics::Vector2 {
                X: 0.0,
                Y: h1 + LINE_GAP * scale,
            },
            &layout2,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        target.EndDraw(None, None)?;
    }
    drop(target); // release the DC binding before the DIB is destroyed

    let mut pixels = dib.pixels();
    drop(dib);
    hit_testable(&mut pixels);
    Ok(Rendered {
        pixels,
        width,
        height,
    })
}

/// Loads both bundled fonts into a private collection (ADR-0016 point 4). The font files come
/// from the in-memory loader; the collection is built with the Win10 1809 font-set API, which
/// needs no custom `IDWriteFontCollectionLoader` implementation.
fn build_collection(dw: &IDWriteFactory5) -> anyhow::Result<IDWriteFontCollection> {
    // SAFETY: the in-memory loader is a DirectWrite factory object with no preconditions.
    let loader: IDWriteInMemoryFontFileLoader = unsafe { dw.CreateInMemoryFontFileLoader()? };
    // SAFETY: the loader outlives both references below; the font data is static.
    unsafe { dw.RegisterFontFileLoader(&loader)? };
    let base: IDWriteFactory = dw.cast()?;
    // SAFETY: the font bytes are `static`, so they outlive the loader's references.
    let en = unsafe {
        loader.CreateInMemoryFontFileReference(
            &base,
            NOTO_SANS.as_ptr().cast(),
            NOTO_SANS.len() as u32,
            None,
        )?
    };
    // SAFETY: the font bytes are `static`, so they outlive the loader's references.
    let ne = unsafe {
        loader.CreateInMemoryFontFileReference(
            &base,
            NOTO_DEVANAGARI.as_ptr().cast(),
            NOTO_DEVANAGARI.len() as u32,
            None,
        )?
    };

    let dw3: IDWriteFactory3 = dw.cast()?;
    // SAFETY: the builder is a fresh DirectWrite object.
    let builder: IDWriteFontSetBuilder = unsafe { dw3.CreateFontSetBuilder()? };
    // SAFETY: the face references keep the font files alive; the family-name property makes each
    // font findable by `CreateTextFormat`, and the variable font's axis still serves every weight.
    unsafe {
        let en_props = [DWRITE_FONT_PROPERTY {
            propertyId: DWRITE_FONT_PROPERTY_ID_FAMILY_NAME,
            propertyValue: w!("Noto Sans"),
            localeName: w!(""),
        }];
        let ne_props = [DWRITE_FONT_PROPERTY {
            propertyId: DWRITE_FONT_PROPERTY_ID_FAMILY_NAME,
            propertyValue: w!("Noto Sans Devanagari"),
            localeName: w!(""),
        }];
        let en_ref: IDWriteFontFaceReference =
            dw3.CreateFontFaceReference(&en, 0, DWRITE_FONT_SIMULATIONS_NONE)?;
        builder.AddFontFaceReference(&en_ref, &en_props)?;
        let ne_ref: IDWriteFontFaceReference =
            dw3.CreateFontFaceReference(&ne, 0, DWRITE_FONT_SIMULATIONS_NONE)?;
        builder.AddFontFaceReference(&ne_ref, &ne_props)?;
    }
    // SAFETY: the font set is freshly built from the two face references above.
    let set: IDWriteFontSet = unsafe { builder.CreateFontSet()? };
    // SAFETY: the font set is valid; the collection owns its own reference to it.
    let collection1 = unsafe { dw3.CreateFontCollectionFromFontSet(&set)? };
    let collection: IDWriteFontCollection = collection1.cast()?;
    Ok(collection)
}

/// A fallback that maps the Devanagari block to Noto Sans Devanagari; everything else stays in the
/// primary Noto Sans, so DirectWrite shapes each run with the right font (ADR-0016 point 4).
fn devanagari_fallback(
    dw: &IDWriteFactory5,
    collection: &IDWriteFontCollection,
) -> anyhow::Result<IDWriteFontFallback> {
    // SAFETY: the builder is a fresh DirectWrite object.
    let builder: IDWriteFontFallbackBuilder = unsafe { dw.CreateFontFallbackBuilder()? };
    let range = [DWRITE_UNICODE_RANGE {
        first: 0x0900,
        last: 0x097F,
    }];
    let family = w!("Noto Sans Devanagari");
    // SAFETY: the strings and the collection outlive the call; the mapping is static.
    unsafe {
        builder.AddMapping(
            &range,
            &[family.0],
            collection,
            w!("en-US"),
            w!("Noto Sans"),
            1.0,
        )?;
    }
    // SAFETY: the builder is freshly created and valid.
    let fallback: IDWriteFontFallback = unsafe { builder.CreateFontFallback()? };
    Ok(fallback)
}

/// A text format for one line: family Noto Sans, weight per line, 12 px at the taskbar scale, with
/// the Devanagari fallback applied so the layouts it produces shape both scripts.
fn text_format(
    dw: &IDWriteFactory5,
    collection: &IDWriteFontCollection,
    weight: DWRITE_FONT_WEIGHT,
    scale: f32,
    fallback: &IDWriteFontFallback,
) -> anyhow::Result<IDWriteTextFormat> {
    // SAFETY: the family name and locale are static; the collection outlives the format.
    let format = unsafe {
        dw.CreateTextFormat(
            w!("Noto Sans"),
            Some(collection),
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            FONT_SIZE * scale,
            w!("en-US"),
        )
    }
    .context("CreateTextFormat")?;
    // SAFETY: the format is fresh; SetFontFallback lives on IDWriteTextFormat1.
    let format1: IDWriteTextFormat1 = format.cast()?;
    // SAFETY: the fallback outlives the format.
    unsafe { format1.SetFontFallback(fallback)? };
    Ok(format)
}

/// Lays out one line for measuring and drawing, with no wrapping.
fn layout(
    dw: &IDWriteFactory5,
    line: &str,
    format: &IDWriteTextFormat,
) -> anyhow::Result<IDWriteTextLayout> {
    let text: Vec<u16> = line.encode_utf16().collect();
    // SAFETY: `text` outlives the call; the format is valid.
    let layout = unsafe { dw.CreateTextLayout(&text, format, 10_000.0, 10_000.0)? };
    // SAFETY: the layout is freshly created and valid.
    unsafe { layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)? };
    Ok(layout)
}

/// The laid-out width and height, the physical size the text needs.
fn metrics(layout: &IDWriteTextLayout) -> anyhow::Result<(f32, f32)> {
    let mut metrics = DWRITE_TEXT_METRICS::default();
    // SAFETY: `metrics` is a valid out struct.
    unsafe { layout.GetMetrics(&mut metrics)? };
    Ok((metrics.width, metrics.height))
}

/// A 32-bit top-down DIB with a compatible DC selected into it. Dropping it restores the previous
/// bitmap and deletes the GDI objects.
struct Dib {
    hdc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut std::ffi::c_void,
    width: u32,
    height: u32,
}

impl Dib {
    fn new(width: u32, height: u32) -> anyhow::Result<Self> {
        // SAFETY: the screen DC is released right after the compatible DC is created.
        let screen = unsafe { GetDC(None) };
        if screen.is_invalid() {
            bail!("GetDC returned a null DC");
        }
        // SAFETY: `screen` is a valid DC.
        let hdc = unsafe { CreateCompatibleDC(Some(screen)) };
        // SAFETY: the screen DC is no longer needed.
        unsafe { ReleaseDC(None, screen) };
        if hdc.is_invalid() {
            bail!("CreateCompatibleDC returned a null DC");
        }

        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // negative: top-down (ADR-0016 point 4)
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            bmiColors: [RGBQUAD::default(); 1],
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: `info` describes the DIB; `bits` receives the pixel memory.
        let bitmap =
            unsafe { CreateDIBSection(Some(hdc), &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
        // SAFETY: selecting the DIB makes the DC draw into `bits`; the old object is restored in
        // `Drop`.
        let old = unsafe { SelectObject(hdc, HGDIOBJ(bitmap.0)) };
        Ok(Dib {
            hdc,
            bitmap,
            old,
            bits,
            width,
            height,
        })
    }

    /// Copies the premultiplied BGRA pixels out of the DIB.
    fn pixels(&self) -> Vec<u8> {
        // SAFETY: `bits` points at `width * height * 4` bytes written by the render target.
        unsafe {
            std::slice::from_raw_parts(
                self.bits.cast::<u8>(),
                self.width as usize * self.height as usize * 4,
            )
            .to_vec()
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        // SAFETY: the objects are exactly those created in `Dib::new`.
        unsafe {
            SelectObject(self.hdc, self.old);
            let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _ = DeleteDC(self.hdc);
        }
    }
}

/// Ensures every pixel has at least alpha 1 so the whole rectangle takes clicks (ADR-0016 point
/// 4). Fully transparent pixels stay visually transparent (alpha 1 is invisible) but stop the
/// click from falling through to the taskbar.
fn hit_testable(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        if pixel[3] == 0 {
            pixel[3] = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::windows::theme;
    use windows::Win32::Graphics::DirectWrite::DWRITE_CLUSTER_METRICS;

    /// Serializes the render tests: each one registers a ~13 MB font loader in the process on its
    /// thread, so the process-wide memory test would see another thread's one-time load as a leak.
    static RENDER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn render_guard() -> std::sync::MutexGuard<'static, ()> {
        RENDER_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn lines() -> TaskbarLines {
        TaskbarLines {
            line1: "Thursday | 1:50 PM".to_owned(),
            line2: "Ashoj 29, 2083 | Oct 15, 2026".to_owned(),
        }
    }

    #[test]
    fn render_is_premultiplied_and_hit_testable() {
        let _guard = render_guard();
        let rendered = render(&lines(), 1.0, theme::INK_LIGHT).expect("render");
        assert!(rendered.width > 0 && rendered.height > 0);
        assert_eq!(
            rendered.pixels.len(),
            rendered.width as usize * rendered.height as usize * 4
        );
        let mut opaque = 0;
        for pixel in rendered.pixels.as_chunks::<4>().0 {
            let (b, g, r, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
            assert!(
                a >= r.max(g).max(b),
                "pixel not premultiplied: rgba={r},{g},{b},{a}"
            );
            assert!(a >= 1, "pixel has zero alpha");
            if a == 255 {
                opaque += 1;
            }
        }
        assert!(opaque > 0, "no opaque text pixels");
    }

    #[test]
    fn render_width_is_reasonable() {
        let _guard = render_guard();
        let rendered = render(&lines(), 1.0, theme::INK_LIGHT).expect("render");
        // The two English lines are around 170 px wide; a much smaller width means the Latin
        // glyphs did not shape.
        assert!(
            rendered.width > 100,
            "rendered width too small: {}",
            rendered.width
        );
        assert!(rendered.height > 20, "rendered height too small");
    }

    #[test]
    fn render_twice_is_stable() {
        let _guard = render_guard();
        let first = render(&lines(), 1.0, theme::INK_LIGHT).expect("render");
        let second = render(&lines(), 1.0, theme::INK_LIGHT).expect("render");
        assert_eq!(first.width, second.width, "render width not stable");
        assert!(first.width > 100, "width too small: {}", first.width);
    }

    #[test]
    fn render_does_not_leak_font_loaders() {
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows::Win32::System::Threading::GetCurrentProcess;

        // windows-rs 0.62 models PROCESS_MEMORY_COUNTERS without its last field (PrivateUsage),
        // so define the full layout here and cast to the truncated type.
        #[repr(C)]
        #[derive(Default)]
        struct MemoryCounters {
            cb: u32,
            page_fault_count: u32,
            peak_working_set_size: usize,
            working_set_size: usize,
            quota_peak_paged_pool_usage: usize,
            quota_paged_pool_usage: usize,
            quota_peak_non_paged_pool_usage: usize,
            quota_non_paged_pool_usage: usize,
            pagefile_usage: usize,
            peak_pagefile_usage: usize,
            private_usage: usize,
        }

        fn private_bytes() -> usize {
            let mut counters = MemoryCounters {
                cb: std::mem::size_of::<MemoryCounters>() as u32,
                ..Default::default()
            };
            // SAFETY: `counters` has the real PROCESS_MEMORY_COUNTERS layout and `cb` is its full
            // byte size, so the Win32 function writes every field, including `private_usage`.
            unsafe {
                GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    std::ptr::from_mut(&mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
                    counters.cb,
                )
                .expect("GetProcessMemoryInfo");
            }
            counters.private_usage
        }

        let _guard = render_guard();
        // The first render pays the one-time font load; the next 1000 must not add a loader.
        render(&lines(), 1.0, theme::INK_LIGHT).expect("warm render");
        let before = private_bytes();
        for _ in 0..1000 {
            render(&lines(), 1.0, theme::INK_LIGHT).expect("render");
        }
        let after = private_bytes();
        let growth = after.saturating_sub(before);
        assert!(
            growth < 1_000_000,
            "font loaders leaked: private bytes grew {before} -> {after} ({growth} bytes)"
        );
    }

    #[test]
    fn render_shapes_devanagari() {
        let _guard = render_guard();
        // `क्ष` is three code points (क ् ष) but one glyph cluster once shaped; a Latin-only font
        // would keep it as three clusters. The cluster count is DirectWrite's glyph count.
        // SAFETY: DirectWrite is a process-wide COM API; a shared factory is the documented default.
        let dw: IDWriteFactory5 =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap() };
        let collection = build_collection(&dw).unwrap();
        let fallback = devanagari_fallback(&dw, &collection).unwrap();
        let format =
            text_format(&dw, &collection, DWRITE_FONT_WEIGHT_REGULAR, 1.0, &fallback).unwrap();
        let layout = layout(&dw, "क्ष", &format).unwrap();
        let mut buffer = [DWRITE_CLUSTER_METRICS::default(); 16];
        let mut clusters = 0u32;
        // SAFETY: `buffer` is large enough for the three code points and `clusters` is a valid
        // out value.
        unsafe {
            layout
                .GetClusterMetrics(Some(&mut buffer), &mut clusters)
                .expect("GetClusterMetrics");
        }
        let code_points: Vec<char> = "क्ष".chars().collect();
        assert_eq!(code_points.len(), 3);
        assert!(
            clusters < code_points.len() as u32,
            "क्ष should shape into fewer clusters than code points, got {clusters}"
        );
    }
}
