//! Scan results shared by the screen overlay and the preview window: classification,
//! pill labels, the "what happens on Enter" rule, history, sound and auto-copy.

use qrtool_core::history::{History, ScanSource};
use qrtool_core::payload::{classify, Payload};
use qrtool_core::settings::Settings;
use qrtool_core::text::{sanitize_for_display, single_line, truncate_chars};
use qrtool_core::url_safety::{self, Severity};
use qrtool_decode::{Detection, Rect};

use crate::ui;

/// One decoded code, ready for display.
#[derive(Debug, Clone)]
pub struct ScanItem {
    /// Decoded text exactly as read.
    pub text: String,
    /// Barcode format name from zxing, e.g. `QRCode`, `EAN13`.
    pub format: String,
    /// Bounding box in source-image pixels.
    pub bounds: Rect,
    pub payload: Payload,
    /// Index of the monitor (screen scans) the code is on; 0 elsewhere.
    pub source: usize,
}

impl ScanItem {
    pub fn from_detection(det: Detection, source: usize) -> Self {
        let payload = classify(&det.text, Some(&det.bytes));
        Self {
            bounds: det.bounds(),
            text: det.text,
            format: det.format,
            payload,
            source,
        }
    }

    /// 0 ok, 1 info, 2 warning, 3 danger (only links are rated).
    pub fn level(&self) -> i32 {
        match &self.payload {
            Payload::Url(r) => r.level as i32,
            Payload::OtherScheme { .. } => Severity::Info as i32,
            _ => 0,
        }
    }

    pub fn is_qr(&self) -> bool {
        self.format.contains("QR")
    }
}

/// Converts detections and joins the two halves of Taiwan e-invoices.
pub fn items_from(detections: Vec<Detection>, source: usize) -> Vec<ScanItem> {
    let mut items: Vec<ScanItem> = detections
        .into_iter()
        .map(|d| ScanItem::from_detection(d, source))
        .collect();
    merge_einvoices(&mut items);
    items
}

/// If a left e-invoice code and a right one (`**…`) were both found, the right one's
/// items are merged into the left one and the right pill is removed.
pub fn merge_einvoices(items: &mut Vec<ScanItem>) {
    let rights: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, it)| matches!(it.payload, Payload::EInvoiceContinuation))
        .map(|(i, _)| i)
        .collect();
    if rights.is_empty() {
        return;
    }
    let mut consumed = Vec::new();
    for li in 0..items.len() {
        let Payload::EInvoice(inv) = &items[li].payload else {
            continue;
        };
        if inv.has_right_part {
            continue;
        }
        // Pair with the nearest unused right code on the same source.
        let left_center = items[li].bounds.center();
        let best = rights
            .iter()
            .copied()
            .filter(|ri| !consumed.contains(ri) && items[*ri].source == items[li].source)
            .min_by(|a, b| {
                let da = dist(items[*a].bounds.center(), left_center);
                let db = dist(items[*b].bounds.center(), left_center);
                da.total_cmp(&db)
            });
        if let Some(ri) = best {
            let right_text = items[ri].text.clone();
            if let Payload::EInvoice(inv) = &mut items[li].payload {
                inv.merge_right(&right_text);
            }
            consumed.push(ri);
        }
    }
    consumed.sort_unstable_by(|a, b| b.cmp(a));
    for ri in consumed {
        items.remove(ri);
    }
}

fn dist(a: qrtool_decode::Point, b: qrtool_decode::Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Adds `new` items to `items`, skipping codes that are already there (same text at
/// roughly the same place). Returns how many were added.
pub fn add_unique(items: &mut Vec<ScanItem>, new: Vec<ScanItem>) -> usize {
    let mut added = 0;
    for it in new {
        let dup = items.iter().any(|e| {
            e.source == it.source
                && e.text == it.text
                && dist(e.bounds.center(), it.bounds.center()) < e.bounds.w.max(it.bounds.w)
        });
        if !dup {
            items.push(it);
            added += 1;
        }
    }
    if added > 0 {
        merge_einvoices(items);
    }
    added
}

/// Text shown in the UI: no control or bidi characters, one line, limited length.
pub fn display(s: &str, max: usize) -> String {
    truncate_chars(&single_line(&sanitize_for_display(s)), max)
}

/// Main and secondary text of a pill.
pub fn pill_label(item: &ScanItem, s: &ui::Strings<'_>) -> (String, String) {
    let kind = s.invoke_kind_label(item.payload.kind().id().into()).to_string();
    let (label, sub) = match &item.payload {
        Payload::Url(r) => {
            let sub = r
                .findings
                .first()
                .filter(|f| f.severity() >= Severity::Warning)
                .map(|f| {
                    s.invoke_finding(f.code().into(), display(&f.detail(), 60).into())
                        .to_string()
                })
                .unwrap_or_default();
            (display(&r.host_display, 60), sub)
        }
        Payload::Wifi(w) => (display(&w.ssid, 48), kind),
        Payload::Contact(c) => {
            let name = if !c.name.is_empty() {
                c.name.clone()
            } else {
                c.phones.first().or(c.emails.first()).cloned().unwrap_or_default()
            };
            (display(&name, 48), kind)
        }
        Payload::Email { to, .. } => (display(to, 48), kind),
        Payload::Phone { number } | Payload::Sms { number, .. } => (display(number, 32), kind),
        Payload::Geo { lat, lon, .. } => (format!("{lat:.5}, {lon:.5}"), kind),
        Payload::Otp { issuer, account } => {
            let name = issuer
                .clone()
                .or_else(|| account.clone())
                .unwrap_or_else(|| kind.clone());
            (display(&name, 48), kind)
        }
        Payload::Crypto { address, .. } => (shorten_middle(address, 20), kind),
        Payload::EInvoice(inv) => (format!("{} · NT${}", inv.number, inv.total_amount), kind),
        Payload::EInvoiceContinuation => (kind, String::new()),
        Payload::OtherScheme { scheme } => (display(&item.text, 48), format!("{kind} ({scheme}:)")),
        Payload::Text => {
            let sub = if item.is_qr() {
                kind
            } else {
                format_label(&item.format)
            };
            (display(&item.text, 48), sub)
        }
    };
    (if label.is_empty() { "—".into() } else { label }, sub)
}

/// Human-friendly barcode format names ("EAN13" → "EAN-13").
pub fn format_label(format: &str) -> String {
    match format {
        "EAN13" => "EAN-13".into(),
        "EAN8" => "EAN-8".into(),
        "UPCA" => "UPC-A".into(),
        "UPCE" => "UPC-E".into(),
        "Code128" => "Code 128".into(),
        "Code39" => "Code 39".into(),
        "Code93" => "Code 93".into(),
        "DataMatrix" => "Data Matrix".into(),
        "QRCode" => "QR Code".into(),
        "MicroQRCode" => "Micro QR Code".into(),
        "RMQRCode" => "rMQR Code".into(),
        other => other.to_string(),
    }
}

fn shorten_middle(s: &str, keep: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= keep * 2 + 1 {
        return display(s, keep * 2 + 1);
    }
    let head: String = chars[..keep].iter().collect();
    let tail: String = chars[chars.len() - keep / 2..].iter().collect();
    display(&format!("{head}…{tail}"), keep * 2)
}

/// What Enter (or a click on a pill) does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primary {
    /// Open the link right away (clean or merely informational links).
    Open,
    /// Show the detail card first (warnings, and everything that is not a web link).
    Details,
}

pub fn primary_action(item: &ScanItem) -> Primary {
    match &item.payload {
        Payload::Url(r) if r.level < Severity::Warning => Primary::Open,
        _ => Primary::Details,
    }
}

/// The single item that may be opened without asking, if any (one code, trusted site,
/// nothing suspicious, and the user allows it).
pub fn auto_open_candidate(items: &[ScanItem], settings: &Settings) -> Option<usize> {
    if items.len() != 1 {
        return None;
    }
    match &items[0].payload {
        Payload::Url(r)
            if url_safety::may_auto_open(
                r,
                settings.scan.auto_open_trusted,
                &settings.scan.trusted_domains,
            ) =>
        {
            Some(0)
        }
        _ => None,
    }
}

/// Opens an item with the matching system handler. Only `http`/`https` links reach the
/// browser; other schemes are never handed to the system.
pub fn open_item(item: &ScanItem) -> Result<(), String> {
    use qrtool_platform::open;
    let res = match &item.payload {
        Payload::Url(r) => open::open_web(r),
        Payload::Email { to, subject, body } => open::open_mail(to, subject.as_deref(), body.as_deref()),
        Payload::Phone { number } => open::open_phone(number),
        Payload::Sms { number, body } => open::open_sms(number, body.as_deref()),
        Payload::Geo { lat, lon, .. } => open::open_map(*lat, *lon),
        _ => return Err("not openable".into()),
    };
    res.map_err(|e| e.to_string())
}

/// Things that happen once when codes are found: sound, auto-copy and history.
pub fn on_found(items: &[ScanItem], source: ScanSource, settings: &Settings) {
    if items.is_empty() {
        return;
    }
    if settings.scan.sound {
        qrtool_platform::sound::play_found();
    }
    if settings.scan.auto_copy && items.len() == 1 && !items[0].payload.is_sensitive() {
        qrtool_platform::clipboard::set_text(&items[0].text);
    }
    record_history(items, source, settings);
}

pub fn record_history(items: &[ScanItem], source: ScanSource, settings: &Settings) {
    if !settings.history.enabled {
        return;
    }
    let history = History::new(
        qrtool_platform::paths::history_file(),
        settings.history.max_entries,
    );
    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    for it in items {
        if let Err(err) = history.record(&it.payload, &it.text, source, ts) {
            tracing::warn!("cannot write history: {err}");
            break;
        }
    }
}

/// Puts `pills` into the UI model. Existing rows are updated in place (so their appear
/// animations do not replay when only the selection changes) and new ones appended; a
/// fresh model is returned only when rows disappeared or there was no model yet.
pub fn update_pills(
    current: slint::ModelRc<ui::PillData>,
    pills: Vec<ui::PillData>,
) -> Option<slint::ModelRc<ui::PillData>> {
    use slint::Model;
    if let Some(vm) = current.as_any().downcast_ref::<slint::VecModel<ui::PillData>>() {
        if pills.len() >= vm.row_count() {
            let old = vm.row_count();
            for (i, p) in pills.into_iter().enumerate() {
                if i < old {
                    if vm.row_data(i).as_ref() != Some(&p) {
                        vm.set_row_data(i, p);
                    }
                } else {
                    vm.push(p);
                }
            }
            return None;
        }
    }
    Some(slint::ModelRc::new(slint::VecModel::from(pills)))
}

/// Builds the pill models for one source. `selected` is a global index into `all`.
pub fn pills_for(
    all: &[ScanItem],
    source: usize,
    selected: Option<usize>,
    s: &ui::Strings<'_>,
    mirror_width: Option<f32>,
) -> Vec<ui::PillData> {
    all.iter()
        .enumerate()
        .filter(|(_, it)| it.source == source)
        .map(|(i, it)| {
            let (label, sublabel) = pill_label(it, s);
            let mut x = it.bounds.x;
            if let Some(w) = mirror_width {
                x = w - it.bounds.x - it.bounds.w;
            }
            ui::PillData {
                x,
                y: it.bounds.y,
                w: it.bounds.w,
                h: it.bounds.h,
                kind: it.payload.kind().id().into(),
                label: label.into(),
                sublabel: sublabel.into(),
                level: it.level(),
                selected: selected == Some(i),
                live: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qrtool_decode::Point;

    fn det(text: &str, x: f32) -> Detection {
        Detection {
            text: text.into(),
            bytes: text.as_bytes().to_vec(),
            format: "QRCode".into(),
            quad: [
                Point { x, y: 0.0 },
                Point { x: x + 10.0, y: 0.0 },
                Point { x: x + 10.0, y: 10.0 },
                Point { x, y: 10.0 },
            ],
        }
    }

    #[test]
    fn clean_links_open_and_others_show_details() {
        let items = items_from(vec![det("https://example.com", 0.0), det("hello", 20.0)], 0);
        assert_eq!(primary_action(&items[0]), Primary::Open);
        assert_eq!(primary_action(&items[1]), Primary::Details);
        let http = items_from(vec![det("http://example.com", 0.0)], 0);
        assert_eq!(primary_action(&http[0]), Primary::Details);
    }

    #[test]
    fn add_unique_skips_duplicates() {
        let mut items = items_from(vec![det("a", 0.0)], 0);
        assert_eq!(add_unique(&mut items, items_from(vec![det("a", 1.0)], 0)), 0);
        assert_eq!(add_unique(&mut items, items_from(vec![det("a", 500.0)], 0)), 1);
        assert_eq!(add_unique(&mut items, items_from(vec![det("a", 0.0)], 1)), 1);
    }

    #[test]
    fn auto_open_requires_trust() {
        let mut settings = Settings::default();
        let items = items_from(vec![det("https://example.com/x", 0.0)], 0);
        assert_eq!(auto_open_candidate(&items, &settings), None);
        settings.scan.trusted_domains.push("example.com".into());
        assert_eq!(auto_open_candidate(&items, &settings), Some(0));
        settings.scan.auto_open_trusted = false;
        assert_eq!(auto_open_candidate(&items, &settings), None);
    }
}
