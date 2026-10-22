//! Direct2D + DirectWrite into a premultiplied BGRA buffer (ADR-0019 point 2, widget-design §5).
//! No window: the caller passes the two lines, the DPI scale, the palette and the hover state, and
//! gets back the pixels of the pill and their size, so the rendering is testable without a
//! display. The bundled Noto fonts are loaded from memory into a private collection, so there is
//! no system-font dependency.
// FFI conventions the renderer cannot avoid: `&x` where `*const T` is expected, and the text
// dimensions, font sizes and DIB metrics, which are all a few hundred pixels or bytes at most.
#![allow(
    clippy::borrow_as_ptr,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use anyhow::{Context, bail};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_PROPERTY, DWRITE_FONT_PROPERTY_ID_FAMILY_NAME,
    DWRITE_FONT_SIMULATIONS_NONE, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_UNICODE_RANGE, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteFactory3, IDWriteFactory5, IDWriteFontCollection,
    IDWriteFontFaceReference, IDWriteFontFallback, IDWriteFontFallbackBuilder, IDWriteFontSet,
    IDWriteFontSetBuilder, IDWriteInMemoryFontFileLoader, IDWriteTextFormat, IDWriteTextFormat1,
    IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, RGBQUAD, ReleaseDC, SelectObject,
};
use windows::core::w;
use windows::core::{IUnknown, Interface as _};

use super::theme::{Palette, SURFACE_ALPHA};
use crate::app::bar::{BarLines, SEPARATOR};

/// The bundled fonts, embedded at compile time (ADR-0010, ADR-0019 point 2).
const NOTO_SANS: &[u8] = include_bytes!("../../../ui/fonts/NotoSans-Variable.ttf");
const NOTO_DEVANAGARI: &[u8] = include_bytes!("../../../ui/fonts/NotoSansDevanagari-Variable.ttf");

/// widget-design.md §5: line 1 regular, line 2 semibold, 12 px at 100 % scale, 2 px line gap,
/// 10 px padding left and right, 2 px top and bottom, and a 26 px pill for one line.
const FONT_SIZE: f32 = 12.0;
const LINE_GAP: f32 = 2.0;
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 2.0;
/// Corner radius at 100 % scale: a soft rectangle, not a capsule, to sit with the Windows 11 shell.
const RADIUS: f32 = 6.0;
const ONE_LINE_HEIGHT: f32 = 26.0;

/// A rendered buffer: premultiplied BGRA, top-down, one byte per channel.
pub struct Rendered {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Renders the bar's pill at `scale` (dpi / 96) in `palette`; `hover` raises the surface to
/// `surface-raised` (widget-design §5). Every pixel gets at least alpha 1, so the whole rectangle
/// takes clicks (ADR-0016 point 4, kept by ADR-0019 point 2). The DirectWrite and Direct2D
/// factories, the font loader and the font collection are built once per thread.
pub fn render(
    lines: &BarLines,
    scale: f32,
    palette: &Palette,
    hover: bool,
) -> anyhow::Result<Rendered> {
    FONTS.with(|cell| {
        if cell.get().is_none() {
            // A failed init leaves the cell empty, so the next render tries again.
            let _ = cell.set(init_fonts()?);
        }
        let fonts = cell.get().context("font cache is empty after init")?;
        render_with(fonts, lines, scale, palette, hover)
    })
}

/// The shared DirectWrite and Direct2D state for this thread. The bar's thread is the only
/// render caller; the `--bar-snapshot` tool reuses the same cache.
struct Fonts {
    dw: IDWriteFactory5,
    d2d: ID2D1Factory,
    collection: IDWriteFontCollection,
    fallback: IDWriteFontFallback,
}

thread_local! {
    static FONTS: std::cell::OnceCell<Fonts> = const { std::cell::OnceCell::new() };
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

/// One laid-out line: the layout, its measured size and where its separators are.
struct Line {
    layout: IDWriteTextLayout,
    width: f32,
    height: f32,
    separators: Vec<DWRITE_TEXT_RANGE>,
}

/// A straight-alpha Direct2D colour from an `[r, g, b]` token.
fn colour(rgb: [u8; 3], alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: f32::from(rgb[0]) / 255.0,
        g: f32::from(rgb[1]) / 255.0,
        b: f32::from(rgb[2]) / 255.0,
        a: alpha,
    }
}

/// Where each ` · ` separator sits in `text`, in UTF-16 units, which is what DirectWrite counts.
fn separator_ranges(text: &str) -> Vec<DWRITE_TEXT_RANGE> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let separator: Vec<u16> = SEPARATOR.encode_utf16().collect();
    let mut ranges = Vec::new();
    let mut at = 0;
    while at + separator.len() <= units.len() {
        if units[at..at + separator.len()] == separator[..] {
            ranges.push(DWRITE_TEXT_RANGE {
                startPosition: at as u32,
                length: separator.len() as u32,
            });
            at += separator.len();
        } else {
            at += 1;
        }
    }
    ranges
}

/// The render body, using the cached factories, collection and fallback.
// Drawing one pill is one linear sequence of Direct2D calls; splitting it would scatter the
// order the calls have to run in.
#[allow(clippy::too_many_lines)]
fn render_with(
    fonts: &Fonts,
    lines: &BarLines,
    scale: f32,
    palette: &Palette,
    hover: bool,
) -> anyhow::Result<Rendered> {
    let dw = &fonts.dw;
    let collection = &fonts.collection;
    let fallback = &fonts.fallback;

    // A line with nothing to show is not drawn; the other is then the only line. Each line keeps
    // its weight: the top is regular and the bottom semibold.
    let mut rows: Vec<Line> = Vec::new();
    for (text, weight) in [
        (&lines.top, DWRITE_FONT_WEIGHT_REGULAR),
        (&lines.bottom, DWRITE_FONT_WEIGHT_SEMI_BOLD),
    ] {
        if text.is_empty() {
            continue;
        }
        let format = text_format(dw, collection, weight, scale, fallback)?;
        let layout = layout(dw, text, &format)?;
        let (width, height) = metrics(&layout)?;
        rows.push(Line {
            layout,
            width,
            height,
            separators: separator_ranges(text),
        });
    }
    if rows.is_empty() {
        bail!("the bar has no text to draw");
    }

    let gap = LINE_GAP * scale;
    let text_width = rows.iter().map(|row| row.width).fold(0.0, f32::max);
    let text_height =
        rows.iter().map(|row| row.height).sum::<f32>() + gap * (rows.len() - 1) as f32;
    let pad_x = (PAD_X * scale).round();
    let pad_y = (PAD_Y * scale).round();
    let min_height = if rows.len() == 1 {
        (ONE_LINE_HEIGHT * scale).round()
    } else {
        0.0
    };
    let width = (text_width.ceil() + 2.0 * pad_x).max(1.0) as u32;
    let height = (text_height.ceil() + 2.0 * pad_y).max(min_height).max(1.0) as u32;

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
    // SAFETY: grayscale is the right antialiasing on a surface with per-pixel alpha, where
    // ClearType would fringe (ADR-0016 point 4).
    unsafe { target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE) };
    let fill = if hover {
        palette.surface_raised
    } else {
        palette.surface
    };
    // SAFETY: the brushes are used only while the target is alive.
    let (fill_brush, border_brush, ink_brush, muted_brush) = unsafe {
        (
            target.CreateSolidColorBrush(&colour(fill, SURFACE_ALPHA), None)?,
            target.CreateSolidColorBrush(&colour(palette.hairline, 1.0), None)?,
            target.CreateSolidColorBrush(&colour(palette.ink, 1.0), None)?,
            target.CreateSolidColorBrush(&colour(palette.ink_muted, 1.0), None)?,
        )
    };
    // The separators are `ink-muted`; every other character takes the default `ink` brush.
    let muted: IUnknown = muted_brush.cast()?;
    for row in &rows {
        for range in &row.separators {
            // SAFETY: the range lies inside the text the layout was built from; the brush
            // outlives the draw below.
            unsafe { row.layout.SetDrawingEffect(&muted, *range)? };
        }
    }

    let (w, h) = (width as f32, height as f32);
    let radius = (RADIUS * scale).round().min(h / 2.0);
    let pill = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        },
        radiusX: radius,
        radiusY: radius,
    };
    // The 1 px hairline is drawn inside the pill, centred on a line half a pixel in.
    let border = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: 0.5,
            top: 0.5,
            right: w - 0.5,
            bottom: h - 0.5,
        },
        radiusX: radius - 0.5,
        radiusY: radius - 0.5,
    };

    // SAFETY: the layouts and brushes outlive the draw; the origins stay inside the DIB.
    unsafe {
        target.BeginDraw();
        target.Clear(None);
        target.FillRoundedRectangle(&pill, &fill_brush);
        target.DrawRoundedRectangle(&border, &border_brush, 1.0, None);
        let mut y = (h - text_height) / 2.0;
        for row in &rows {
            target.DrawTextLayout(
                windows_numerics::Vector2 { X: pad_x, Y: y },
                &row.layout,
                &ink_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
            y += row.height + gap;
        }
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

    fn lines() -> BarLines {
        BarLines {
            top: "Thu · 6:31 PM · Singapore 8:46 PM".to_owned(),
            bottom: "Kartik 5, 2083 · Oct 22".to_owned(),
        }
    }

    fn nepali_lines() -> BarLines {
        BarLines {
            top: "बिहि · ६:३१ PM · Singapore ८:४६ PM".to_owned(),
            bottom: "कार्तिक ५, २०८३ · Oct २२".to_owned(),
        }
    }

    fn draw(lines: &BarLines, scale: f32, hover: bool) -> Rendered {
        render(lines, scale, &theme::LIGHT, hover).expect("render")
    }

    /// The straight-alpha premultiplied pixel at `(x, y)`: `[b, g, r, a]`.
    fn pixel(rendered: &Rendered, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * rendered.width + x) * 4) as usize;
        [
            rendered.pixels[at],
            rendered.pixels[at + 1],
            rendered.pixels[at + 2],
            rendered.pixels[at + 3],
        ]
    }

    #[test]
    fn render_is_premultiplied_and_hit_testable() {
        let _guard = render_guard();
        let rendered = draw(&lines(), 1.0, false);
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
        let rendered = draw(&lines(), 1.0, false);
        // The widest English line is about 200 px; a much smaller width means the Latin glyphs
        // did not shape.
        assert!(
            rendered.width > 150,
            "rendered width too small: {}",
            rendered.width
        );
    }

    #[test]
    fn render_twice_is_stable() {
        let _guard = render_guard();
        let first = draw(&lines(), 1.0, false);
        let second = draw(&lines(), 1.0, false);
        assert_eq!(first.width, second.width, "render width not stable");
        assert_eq!(first.pixels, second.pixels, "render pixels not stable");
    }

    /// widget-design §5: two lines stay under 44 px so they fit a 48 px taskbar, in both scripts.
    #[test]
    fn wr08_ac7_two_lines_stay_under_44_px_at_100_percent() {
        let _guard = render_guard();
        for (name, lines) in [("en", lines()), ("ne", nepali_lines())] {
            let rendered = draw(&lines, 1.0, false);
            eprintln!(
                "two {name} lines: {} x {} px",
                rendered.width, rendered.height
            );
            assert!(
                rendered.height < 44,
                "two {name} lines are {} px high",
                rendered.height
            );
            assert!(rendered.height >= 30, "two {name} lines are too short");
        }
    }

    /// widget-design §5: one line gives a 26 px pill.
    #[test]
    fn wr08_ac7_one_line_is_a_26_px_pill_and_the_other_line_is_enough() {
        let _guard = render_guard();
        let top_only = BarLines {
            bottom: String::new(),
            ..lines()
        };
        let bottom_only = BarLines {
            top: String::new(),
            ..lines()
        };
        assert_eq!(draw(&top_only, 1.0, false).height, 26);
        assert_eq!(draw(&bottom_only, 1.0, false).height, 26);
        assert_eq!(draw(&top_only, 2.0, false).height, 52);
        assert_eq!(draw(&top_only, 1.5, false).height, 39);
    }

    #[test]
    fn wr08_ac7_the_pill_is_padded_10_px_and_scales_with_dpi() {
        let _guard = render_guard();
        let one = draw(&lines(), 1.0, false);
        let two = draw(&lines(), 2.0, false);
        // Text width plus 2 x 10 px of padding; at 200 % everything doubles, so the width does
        // within a pixel of rounding.
        assert!(
            two.width.abs_diff(one.width * 2) <= 4,
            "{} px at 100 %, {} px at 200 %",
            one.width,
            two.width
        );
        assert!(two.height.abs_diff(one.height * 2) <= 2);
    }

    #[test]
    fn wr08_ac7_the_surface_is_90_percent_opaque_with_round_ends_and_a_hairline() {
        let _guard = render_guard();
        let rendered = draw(&lines(), 1.0, false);
        let middle = rendered.width / 2;
        // Row 0 is the 1 px hairline, fully opaque.
        assert_eq!(pixel(&rendered, middle, 0)[3], 255);
        // Row 1 is the surface at 90 %: alpha 230 (0.9 x 255 = 229.5).
        let surface = pixel(&rendered, middle, 1);
        assert!(
            surface[3].abs_diff(230) <= 2,
            "surface alpha {} is not 90 %",
            surface[3]
        );
        // The corner is outside the pill: it keeps alpha 1 to take clicks, and shows nothing.
        let corner = pixel(&rendered, 0, 0);
        assert_eq!(corner, [0, 0, 0, 1]);
        let last = pixel(&rendered, rendered.width - 1, rendered.height - 1);
        assert_eq!(last, [0, 0, 0, 1]);
    }

    #[test]
    fn wr08_ac7_hover_raises_the_surface() {
        let _guard = render_guard();
        let calm = draw(&lines(), 1.0, false);
        let hover = draw(&lines(), 1.0, true);
        assert_eq!((calm.width, calm.height), (hover.width, hover.height));
        let middle = calm.width / 2;
        // `surface-raised` is white in the light theme, so the blue channel rises from 0xF8.
        let (b_calm, b_hover) = (pixel(&calm, middle, 1)[0], pixel(&hover, middle, 1)[0]);
        assert!(b_hover > b_calm, "hover {b_hover} is not above {b_calm}");
    }

    #[test]
    fn wr08_ac2_the_separator_is_dimmer_than_the_text() {
        let _guard = render_guard();
        // A bare dot has no separator around it, so it is `ink`; with the spaces it is the
        // separator, which is `ink-muted`. At 300 % the dot covers whole pixels, so the darkest
        // opaque pixel of each image is the colour the dot was drawn in.
        let darkest = |top: &str| {
            let rendered = draw(
                &BarLines {
                    top: top.to_owned(),
                    bottom: String::new(),
                },
                3.0,
                false,
            );
            let mut darkest = 255u8;
            // Skip the hairline rows and columns, which are opaque too.
            for y in 3..rendered.height - 3 {
                for x in 3..rendered.width - 3 {
                    let [b, _, _, a] = pixel(&rendered, x, y);
                    if a == 255 {
                        darkest = darkest.min(b);
                    }
                }
            }
            darkest
        };
        let (ink, muted) = (darkest("·"), darkest(" · "));
        assert_eq!(ink, theme::LIGHT.ink[2], "the bare dot is drawn in `ink`");
        assert_eq!(
            muted,
            theme::LIGHT.ink_muted[2],
            "the separator is drawn in `ink-muted`"
        );
    }

    #[test]
    fn separator_ranges_find_every_separator_in_utf16_units() {
        assert!(separator_ranges("Thu").is_empty());
        let ranges = separator_ranges("Thu · 6:31 PM · Singapore");
        let spans: Vec<(u32, u32)> = ranges.iter().map(|r| (r.startPosition, r.length)).collect();
        assert_eq!(spans, vec![(3, 3), (13, 3)]);
        // Devanagari is one UTF-16 unit per scalar here, so offsets still count units.
        let spans: Vec<u32> = separator_ranges("बिहि · ६")
            .iter()
            .map(|r| r.startPosition)
            .collect();
        assert_eq!(spans, vec![4]);
    }

    #[test]
    fn an_empty_bar_is_an_error_not_a_zero_size_window() {
        let _guard = render_guard();
        assert!(render(&BarLines::default(), 1.0, &theme::LIGHT, false).is_err());
    }

    #[test]
    fn render_does_not_leak_font_loaders() {
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
        };
        use windows::Win32::System::Threading::GetCurrentProcess;

        fn private_bytes() -> usize {
            let mut counters = PROCESS_MEMORY_COUNTERS_EX {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                ..Default::default()
            };
            // SAFETY: GetProcessMemoryInfo takes the _EX layout through the base pointer type when
            // `cb` is the _EX size, as its documentation describes; `counters` lives for the call.
            unsafe {
                GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    std::ptr::from_mut(&mut counters).cast::<PROCESS_MEMORY_COUNTERS>(),
                    counters.cb,
                )
                .expect("GetProcessMemoryInfo");
            }
            counters.PrivateUsage
        }

        let _guard = render_guard();
        // The first render pays the one-time font load; the next 1000 must not add a loader.
        draw(&lines(), 1.0, false);
        let before = private_bytes();
        for _ in 0..1000 {
            draw(&lines(), 1.0, false);
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
