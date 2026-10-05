//! Barcode decoding and QR code generation for **QRcode tool**.
//!
//! Decoding uses [zxing-cpp](https://github.com/zxing-cpp/zxing-cpp). On top of a single
//! zxing pass we add the preprocessing that matters for *screen* content:
//!
//! 1. full-image pass (zxing itself tries inverted, rotated and downscaled variants);
//! 2. if nothing was found, a 2× upscaled pass (tiny on-screen codes);
//! 3. if still nothing and the image is large, overlapping tiles, each upscaled.
//!
//! Results are de-duplicated and their positions are always reported in the coordinate
//! system of the image that was passed in.

pub mod generate;

use image::{imageops, GrayImage, RgbaImage};
use zxingcpp::{BarcodeFormat, BarcodeReader, ImageFormat, ImageView};

pub use generate::{generate_qr, EcLevel, QrMatrix};

/// A 2D point in image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// An axis-aligned rectangle in image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }
    /// Clamps the rectangle to `0..width × 0..height`.
    pub fn clamp_to(&self, width: u32, height: u32) -> Rect {
        let x0 = self.x.max(0.0).min(width as f32);
        let y0 = self.y.max(0.0).min(height as f32);
        let x1 = (self.x + self.w).max(0.0).min(width as f32);
        let y1 = (self.y + self.h).max(0.0).min(height as f32);
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }
}

/// One decoded barcode.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Decoded text (UTF-8, as interpreted by zxing).
    pub text: String,
    /// Raw payload bytes (used for e.g. Big5 re-decoding).
    pub bytes: Vec<u8>,
    /// Barcode format name, e.g. `QRCode`.
    pub format: String,
    /// Corner points: top-left, top-right, bottom-right, bottom-left.
    pub quad: [Point; 4],
}

impl Detection {
    /// Axis-aligned bounding box of the code.
    pub fn bounds(&self) -> Rect {
        let xs = self.quad.iter().map(|p| p.x);
        let ys = self.quad.iter().map(|p| p.y);
        let min_x = xs.clone().fold(f32::INFINITY, f32::min);
        let max_x = xs.fold(f32::NEG_INFINITY, f32::max);
        let min_y = ys.clone().fold(f32::INFINITY, f32::min);
        let max_y = ys.fold(f32::NEG_INFINITY, f32::max);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// `true` for the QR code family (QR, Micro QR, rMQR).
    pub fn is_qr(&self) -> bool {
        self.format.contains("QR")
    }

    fn mapped(mut self, scale: f32, dx: f32, dy: f32) -> Self {
        for p in &mut self.quad {
            p.x = p.x / scale + dx;
            p.y = p.y / scale + dy;
        }
        self
    }
}

/// Decoding options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeOptions {
    /// Read every supported symbology, not just the QR family.
    pub all_formats: bool,
    /// Spend more time (rotations, inversions, upscaling, tiling).
    pub thorough: bool,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            all_formats: true,
            thorough: true,
        }
    }
}

impl DecodeOptions {
    /// Fast settings for live camera frames.
    pub fn fast(all_formats: bool) -> Self {
        Self {
            all_formats,
            thorough: false,
        }
    }
}

/// Decoding errors.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("barcode library error: {0}")]
    Library(String),
}

const UPSCALE_LIMIT: u32 = 1600;
const TILE: u32 = 900;
const TILE_OVERLAP: u32 = 250;

fn reader(opts: DecodeOptions) -> BarcodeReader {
    let formats: Vec<BarcodeFormat> = if opts.all_formats {
        vec![BarcodeFormat::AllReadable]
    } else {
        vec![
            BarcodeFormat::QRCode,
            BarcodeFormat::MicroQRCode,
            BarcodeFormat::RMQRCode,
        ]
    };
    BarcodeReader::new()
        .formats(formats)
        .try_harder(opts.thorough)
        .try_rotate(opts.thorough)
        .try_invert(true)
        .try_downscale(true)
        .return_errors(false)
}

/// Single zxing pass over a grayscale image.
pub fn decode_gray(img: &GrayImage, opts: DecodeOptions) -> Result<Vec<Detection>, DecodeError> {
    if img.width() == 0 || img.height() == 0 {
        return Ok(Vec::new());
    }
    let view = ImageView::from_slice(img.as_raw(), img.width(), img.height(), ImageFormat::Lum)
        .map_err(|e| DecodeError::Library(e.to_string()))?;
    let barcodes = reader(opts)
        .from(view)
        .map_err(|e| DecodeError::Library(e.to_string()))?;
    Ok(barcodes
        .into_iter()
        .filter(|b| b.is_valid())
        .map(|b| {
            let p = b.position();
            let pt = |q: zxingcpp::PointI| Point {
                x: q.x as f32,
                y: q.y as f32,
            };
            Detection {
                text: b.text(),
                bytes: b.bytes(),
                format: b.format().to_string(),
                quad: [
                    pt(p.top_left),
                    pt(p.top_right),
                    pt(p.bottom_right),
                    pt(p.bottom_left),
                ],
            }
        })
        .collect())
}

/// Converts RGBA to grayscale (ITU-R BT.601 luma, alpha ignored).
pub fn to_gray(img: &RgbaImage) -> GrayImage {
    imageops::grayscale(img)
}

/// Full multi-pass scan of an RGBA image (screen captures, image files, clipboard).
pub fn scan_image(img: &RgbaImage, opts: DecodeOptions) -> Vec<Detection> {
    scan_gray(&to_gray(img), opts)
}

/// Multi-pass scan of a grayscale image.
pub fn scan_gray(gray: &GrayImage, opts: DecodeOptions) -> Vec<Detection> {
    let mut found = decode_gray(gray, opts).unwrap_or_else(log_err);
    if !found.is_empty() || !opts.thorough {
        return dedupe(found);
    }

    // Pass 2: small codes — upscale the whole image if it is not huge.
    let (w, h) = gray.dimensions();
    if w.max(h) <= UPSCALE_LIMIT {
        found.extend(upscaled_pass(gray, 2, 0.0, 0.0, opts));
        if !found.is_empty() {
            return dedupe(found);
        }
    }

    // Pass 3: overlapping tiles for large images, each tile upscaled.
    if w > TILE || h > TILE {
        for (tx, ty, tw, th) in tiles(w, h) {
            let tile = imageops::crop_imm(gray, tx, ty, tw, th).to_image();
            let mut hits = decode_gray(&tile, opts).unwrap_or_else(log_err);
            if hits.is_empty() {
                hits = upscaled_pass(&tile, 2, 0.0, 0.0, opts);
            }
            found.extend(hits.into_iter().map(|d| d.mapped(1.0, tx as f32, ty as f32)));
        }
    }
    dedupe(found)
}

/// Scans only `region` of `img`. The region is upscaled when small so that codes the user
/// framed tightly are still readable. Positions are returned in `img` coordinates.
pub fn scan_region(img: &RgbaImage, region: Rect, opts: DecodeOptions) -> Vec<Detection> {
    let r = region.clamp_to(img.width(), img.height());
    let (x, y) = (r.x.floor() as u32, r.y.floor() as u32);
    let (w, h) = (r.w.ceil() as u32, r.h.ceil() as u32);
    if w < 4 || h < 4 {
        return Vec::new();
    }
    let crop = imageops::crop_imm(img, x, y, w, h).to_image();
    // Add a white quiet zone: users often frame right on the code's edge.
    let pad = (w.min(h) / 8).clamp(8, 64);
    let mut padded = GrayImage::from_pixel(w + 2 * pad, h + 2 * pad, image::Luma([255]));
    imageops::overlay(&mut padded, &to_gray(&crop), pad as i64, pad as i64);

    let mut found = decode_gray(&padded, opts).unwrap_or_else(log_err);
    if found.is_empty() {
        let factor = if w.max(h) < 300 { 3 } else { 2 };
        if w.max(h) * factor <= 4000 {
            found = upscaled_pass(&padded, factor, 0.0, 0.0, opts);
        }
    }
    if found.is_empty() && opts.thorough {
        found = scan_gray(&padded, opts);
    }
    dedupe(
        found
            .into_iter()
            .map(|d| d.mapped(1.0, x as f32 - pad as f32, y as f32 - pad as f32))
            .collect(),
    )
}

/// Fast single pass for live camera frames (RGBA).
pub fn scan_frame(img: &RgbaImage, all_formats: bool) -> Vec<Detection> {
    dedupe(decode_gray(&to_gray(img), DecodeOptions::fast(all_formats)).unwrap_or_else(log_err))
}

fn upscaled_pass(gray: &GrayImage, factor: u32, dx: f32, dy: f32, opts: DecodeOptions) -> Vec<Detection> {
    let (w, h) = gray.dimensions();
    let big = imageops::resize(gray, w * factor, h * factor, imageops::FilterType::Nearest);
    decode_gray(&big, opts)
        .unwrap_or_else(log_err)
        .into_iter()
        .map(|d| d.mapped(factor as f32, dx, dy))
        .collect()
}

fn tiles(w: u32, h: u32) -> Vec<(u32, u32, u32, u32)> {
    let step = TILE - TILE_OVERLAP;
    let mut out = Vec::new();
    let mut y = 0;
    loop {
        let th = TILE.min(h - y);
        let mut x = 0;
        loop {
            let tw = TILE.min(w - x);
            out.push((x, y, tw, th));
            if x + tw >= w {
                break;
            }
            x += step;
        }
        if y + th >= h {
            break;
        }
        y += step;
    }
    out
}

/// Removes duplicates: the same text whose centres are close is the same physical code.
fn dedupe(mut found: Vec<Detection>) -> Vec<Detection> {
    let mut out: Vec<Detection> = Vec::with_capacity(found.len());
    for d in found.drain(..) {
        let c = d.bounds().center();
        let size = d.bounds().w.max(d.bounds().h).max(1.0);
        let dup = out.iter().any(|o| {
            let oc = o.bounds().center();
            o.text == d.text && (oc.x - c.x).abs() < size && (oc.y - c.y).abs() < size
        });
        if !dup {
            out.push(d);
        }
    }
    // Reading order: top-to-bottom, then left-to-right.
    out.sort_by(|a, b| {
        let (ra, rb) = (a.bounds(), b.bounds());
        let row = |r: &Rect| (r.y / 40.0).floor() as i64;
        row(&ra)
            .cmp(&row(&rb))
            .then(ra.x.partial_cmp(&rb.x).unwrap_or(std::cmp::Ordering::Equal))
    });
    out
}

fn log_err(e: DecodeError) -> Vec<Detection> {
    tracing::warn!("decode error: {e}");
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Luma, Rgba};

    /// Renders `text` as a QR code with `module` pixels per module onto a white canvas.
    fn qr_gray(text: &str, module: u32) -> GrayImage {
        let m = generate_qr(text, EcLevel::M).unwrap();
        let border = 4 * module;
        let size = m.size * module + 2 * border;
        let mut img = GrayImage::from_pixel(size, size, Luma([255]));
        for y in 0..m.size {
            for x in 0..m.size {
                if m.is_dark(x, y) {
                    for dy in 0..module {
                        for dx in 0..module {
                            img.put_pixel(border + x * module + dx, border + y * module + dy, Luma([0]));
                        }
                    }
                }
            }
        }
        img
    }

    fn to_rgba(g: &GrayImage) -> RgbaImage {
        RgbaImage::from_fn(g.width(), g.height(), |x, y| {
            let v = g.get_pixel(x, y).0[0];
            Rgba([v, v, v, 255])
        })
    }

    fn canvas(w: u32, h: u32) -> GrayImage {
        GrayImage::from_pixel(w, h, Luma([255]))
    }

    #[test]
    fn decodes_single_code_with_position() {
        let g = qr_gray("https://github.com/Lrniey666/qrcode-tool", 6);
        let mut c = canvas(800, 600);
        imageops::overlay(&mut c, &g, 300, 200);
        let found = scan_gray(&c, DecodeOptions::default());
        assert_eq!(found.len(), 1);
        let d = &found[0];
        assert_eq!(d.text, "https://github.com/Lrniey666/qrcode-tool");
        assert!(d.is_qr());
        let b = d.bounds();
        // the code (without quiet zone) starts at 300 + 4 modules
        assert!((b.x - (300.0 + 24.0)).abs() < 8.0, "x = {}", b.x);
        assert!((b.y - (200.0 + 24.0)).abs() < 8.0, "y = {}", b.y);
    }

    #[test]
    fn decodes_multiple_codes_in_reading_order() {
        let mut c = canvas(1200, 800);
        for (i, (x, y)) in [(700, 100), (100, 100), (100, 500), (700, 500)]
            .iter()
            .enumerate()
        {
            let g = qr_gray(&format!("code-{i}"), 5);
            imageops::overlay(&mut c, &g, *x, *y);
        }
        let found = scan_gray(&c, DecodeOptions::default());
        let texts: Vec<_> = found.iter().map(|d| d.text.as_str()).collect();
        assert_eq!(texts, vec!["code-1", "code-0", "code-2", "code-3"]);
    }

    #[test]
    fn decodes_inverted_code() {
        let mut g = qr_gray("inverted", 5);
        imageops::invert(&mut g);
        let found = scan_gray(&g, DecodeOptions::default());
        assert_eq!(found.first().map(|d| d.text.as_str()), Some("inverted"));
    }

    #[test]
    fn region_scan_maps_coordinates_back() {
        let g = qr_gray("region", 4);
        let mut c = canvas(1000, 700);
        imageops::overlay(&mut c, &g, 520, 330);
        let rgba = to_rgba(&c);
        let region = Rect::new(500.0, 310.0, g.width() as f32 + 40.0, g.height() as f32 + 40.0);
        let found = scan_region(&rgba, region, DecodeOptions::default());
        assert_eq!(found.len(), 1);
        let b = found[0].bounds();
        assert!((b.x - (520.0 + 16.0)).abs() < 8.0, "x = {}", b.x);
        // tight crop exactly on the code edge still decodes thanks to the padding
        let tight = Rect::new(536.0, 346.0, (g.width() - 32) as f32, (g.height() - 32) as f32);
        assert_eq!(scan_region(&rgba, tight, DecodeOptions::default()).len(), 1);
    }

    #[test]
    fn tiny_code_found_by_upscaling() {
        // 1 pixel per module is too small for a direct pass on a busy large canvas.
        let g = qr_gray("tiny", 1);
        let mut c = canvas(600, 400);
        imageops::overlay(&mut c, &g, 250, 150);
        let found = scan_gray(&c, DecodeOptions::default());
        assert_eq!(found.first().map(|d| d.text.as_str()), Some("tiny"));
    }

    #[test]
    fn empty_and_blank_images() {
        assert!(scan_gray(&canvas(0, 0), DecodeOptions::default()).is_empty());
        assert!(scan_gray(&canvas(2000, 1200), DecodeOptions::default()).is_empty());
    }

    #[test]
    fn utf8_text() {
        let g = qr_gray("Hello 繁體中文測試", 5);
        let found = scan_gray(&g, DecodeOptions::default());
        assert_eq!(found[0].text, "Hello 繁體中文測試");
    }

    #[test]
    fn tiles_cover_image() {
        let t = tiles(2000, 1000);
        assert!(t.iter().all(|(x, y, w, h)| x + w <= 2000 && y + h <= 1000));
        assert!(t.iter().any(|(x, _, w, _)| x + w == 2000));
        assert!(t.iter().any(|(_, y, _, h)| y + h == 1000));
    }
}
