//! Taiwan electronic invoice (電子發票證明聯) QR code parser.
//!
//! A printed e-invoice carries two QR codes side by side:
//!
//! * **Left code** — 77 fixed-width characters followed by `:`-separated fields:
//!
//!   | chars | field |
//!   |---|---|
//!   | 1–10  | invoice number (2 letters + 8 digits) |
//!   | 11–17 | issue date, ROC calendar `yyyMMdd` |
//!   | 18–21 | random code |
//!   | 22–29 | sales amount (untaxed), hexadecimal |
//!   | 30–37 | total amount (taxed), hexadecimal |
//!   | 38–45 | buyer business ID (`00000000` for consumers) |
//!   | 46–53 | seller business ID |
//!   | 54–77 | encrypted verification data |
//!
//!   then `:custom-area:items-in-this-code:total-items:encoding:` and the item list
//!   `name:quantity:unit-price:…`. Encoding is `0` = Big5, `1` = UTF-8, `2` = Base64.
//!
//! * **Right code** — starts with `**` and continues the item list.
//!
//! Source: Ministry of Finance "電子發票證明聯一維及二維條碼規格說明". Real-world
//! printers deviate from the spec, so the parser is lenient about the item section.

use base64::Engine as _;
use serde::{Deserialize, Serialize};

/// One purchased item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceItem {
    pub name: String,
    pub quantity: String,
    pub unit_price: String,
}

/// Parsed left QR code of an e-invoice (optionally merged with the right code).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EInvoice {
    /// Invoice number, e.g. `AB12345678`.
    pub number: String,
    /// Issue date in ISO format (`2026-10-05`), converted from the ROC calendar.
    pub date: String,
    /// The 4-digit random code (隨機碼).
    pub random_code: String,
    /// Sales amount before tax.
    pub sales_amount: u64,
    /// Total amount including tax.
    pub total_amount: u64,
    /// Buyer business ID, `None` for consumer invoices (`00000000`).
    pub buyer_id: Option<String>,
    /// Seller business ID (統一編號).
    pub seller_id: String,
    /// Number of items recorded in the QR codes.
    pub items_in_code: Option<u32>,
    /// Total number of items on the invoice.
    pub total_items: Option<u32>,
    /// Encoding parameter of the item names (0 Big5, 1 UTF-8, 2 Base64).
    pub encoding: Option<u8>,
    /// Parsed items (may be incomplete if the right code was not scanned).
    pub items: Vec<InvoiceItem>,
    /// `true` once the right-hand code has been merged in.
    pub has_right_part: bool,
    /// Raw item section from the left code (kept for merging).
    #[serde(skip)]
    item_section: String,
}

const FIXED_LEN: usize = 77;

/// Returns `true` if `text` looks like the *right* half of an e-invoice (`**…`).
pub fn is_right_part(text: &str) -> bool {
    text.starts_with("**")
}

/// Parses the left QR code. `raw_bytes`, when available, lets us re-decode Big5 item names
/// that the barcode library could not interpret.
pub fn parse_left(text: &str, raw_bytes: Option<&[u8]>) -> Option<EInvoice> {
    let mut inv = parse_left_text(text)?;
    if inv.encoding == Some(0) {
        if let Some(bytes) = raw_bytes {
            let (decoded, _, had_errors) = encoding_rs::BIG5.decode(bytes);
            if !had_errors {
                if let Some(big5) = parse_left_text(&decoded) {
                    inv = big5;
                }
            }
        }
    }
    Some(inv)
}

fn parse_left_text(text: &str) -> Option<EInvoice> {
    let text = text.trim();
    if !text.is_char_boundary(FIXED_LEN.min(text.len())) || text.len() < FIXED_LEN {
        return None;
    }
    let fixed = &text[..FIXED_LEN];
    if !fixed.is_ascii() {
        return None;
    }
    let number = &fixed[0..10];
    let roc_date = &fixed[10..17];
    let random_code = &fixed[17..21];
    let sales_hex = &fixed[21..29];
    let total_hex = &fixed[29..37];
    let buyer = &fixed[37..45];
    let seller = &fixed[45..53];

    let num_ok = number[..2].chars().all(|c| c.is_ascii_uppercase())
        && number[2..].chars().all(|c| c.is_ascii_digit());
    if !num_ok
        || !is_digits(roc_date)
        || !is_digits(random_code)
        || !is_hex(sales_hex)
        || !is_hex(total_hex)
        || !is_digits(buyer)
        || !is_digits(seller)
    {
        return None;
    }
    let date = roc_to_iso(roc_date)?;
    let sales_amount = u64::from_str_radix(sales_hex, 16).ok()?;
    let total_amount = u64::from_str_radix(total_hex, 16).ok()?;

    let rest = &text[FIXED_LEN..];
    let mut items_in_code = None;
    let mut total_items = None;
    let mut encoding = None;
    let mut item_section = String::new();
    if let Some(rest) = rest.strip_prefix(':') {
        // custom-area : items-in-code : total-items : encoding : items...
        let mut parts = rest.splitn(5, ':');
        let _custom = parts.next();
        items_in_code = parts.next().and_then(|s| s.trim().parse().ok());
        total_items = parts.next().and_then(|s| s.trim().parse().ok());
        encoding = parts.next().and_then(|s| s.trim().parse().ok());
        item_section = parts.next().unwrap_or("").to_string();
    }

    let mut inv = EInvoice {
        number: number.to_string(),
        date,
        random_code: random_code.to_string(),
        sales_amount,
        total_amount,
        buyer_id: (buyer != "00000000").then(|| buyer.to_string()),
        seller_id: seller.to_string(),
        items_in_code,
        total_items,
        encoding,
        items: Vec::new(),
        has_right_part: false,
        item_section,
    };
    inv.items = parse_items(&inv.item_section, inv.encoding);
    Some(inv)
}

impl EInvoice {
    /// Merges the right-hand QR code (which starts with `**`) into this invoice.
    pub fn merge_right(&mut self, right: &str) {
        let Some(cont) = right.strip_prefix("**") else {
            return;
        };
        let left = self.item_section.trim_end_matches(':');
        let cont = cont.trim_start_matches(':');
        let joined = match (left.is_empty(), cont.is_empty()) {
            (true, _) => cont.to_string(),
            (_, true) => left.to_string(),
            _ if self.encoding == Some(2) => format!("{left}{cont}"),
            _ => format!("{left}:{cont}"),
        };
        self.item_section = joined;
        self.items = parse_items(&self.item_section, self.encoding);
        self.has_right_part = true;
    }

    /// Tax amount (total − sales).
    pub fn tax_amount(&self) -> u64 {
        self.total_amount.saturating_sub(self.sales_amount)
    }
}

fn parse_items(section: &str, encoding: Option<u8>) -> Vec<InvoiceItem> {
    let decoded;
    let section = if encoding == Some(2) {
        let compact: String = section.chars().filter(|c| !c.is_whitespace()).collect();
        match base64::engine::general_purpose::STANDARD.decode(compact.as_bytes()) {
            Ok(bytes) => {
                decoded = match String::from_utf8(bytes.clone()) {
                    Ok(s) => s,
                    Err(_) => encoding_rs::BIG5.decode(&bytes).0.into_owned(),
                };
                decoded.as_str()
            }
            Err(_) => return Vec::new(),
        }
    } else {
        section
    };
    let fields: Vec<&str> = section.split(':').collect();
    fields
        .chunks(3)
        .filter(|c| c.len() == 3 && !c[0].trim().is_empty())
        .map(|c| InvoiceItem {
            name: c[0].trim().to_string(),
            quantity: c[1].trim().to_string(),
            unit_price: c[2].trim().to_string(),
        })
        .collect()
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Converts ROC `yyyMMdd` to ISO `yyyy-MM-dd` with a basic validity check.
fn roc_to_iso(roc: &str) -> Option<String> {
    let year: i32 = roc[0..3].parse().ok()?;
    let month: u32 = roc[3..5].parse().ok()?;
    let day: u32 = roc[5..7].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year == 0 {
        return None;
    }
    Some(format!("{:04}-{:02}-{:02}", year + 1911, month, day))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed() -> String {
        // number + date(115/10/05) + random + sales(100) + total(105) + buyer + seller + 24 enc
        format!(
            "AB12345678{}{}{}{}{}{}{}",
            "1151005", "1234", "00000064", "00000069", "00000000", "12345678", "ABCDEFGHIJKLMNOPQRSTUVWX"
        )
    }

    #[test]
    fn parses_left_utf8() {
        let s = format!("{}:**********:2:3:1:牛奶:1:55:麵包:1:50", fixed());
        let inv = parse_left(&s, None).unwrap();
        assert_eq!(inv.number, "AB12345678");
        assert_eq!(inv.date, "2026-10-05");
        assert_eq!(inv.random_code, "1234");
        assert_eq!(inv.sales_amount, 100);
        assert_eq!(inv.total_amount, 105);
        assert_eq!(inv.tax_amount(), 5);
        assert_eq!(inv.buyer_id, None);
        assert_eq!(inv.seller_id, "12345678");
        assert_eq!(inv.items_in_code, Some(2));
        assert_eq!(inv.total_items, Some(3));
        assert_eq!(inv.items.len(), 2);
        assert_eq!(inv.items[0].name, "牛奶");
        assert_eq!(inv.items[1].unit_price, "50");
    }

    #[test]
    fn merges_right_part() {
        let s = format!("{}:**********:2:3:1:牛奶:1:55:麵包:1:50", fixed());
        let mut inv = parse_left(&s, None).unwrap();
        assert!(is_right_part("**咖啡:1:45"));
        inv.merge_right("**咖啡:1:45");
        assert!(inv.has_right_part);
        assert_eq!(inv.items.len(), 3);
        assert_eq!(inv.items[2].name, "咖啡");
    }

    #[test]
    fn parses_base64_items() {
        let items = base64::engine::general_purpose::STANDARD.encode("便當:1:80");
        let s = format!("{}:**********:1:1:2:{items}", fixed());
        let inv = parse_left(&s, None).unwrap();
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].name, "便當");
    }

    #[test]
    fn parses_big5_from_raw_bytes() {
        let text = format!("{}:**********:1:1:0:茶:2:30", fixed());
        let (big5, _, _) = encoding_rs::BIG5.encode(&text);
        // Simulate a decoder that mangled the Big5 text.
        let mangled = String::from_utf8_lossy(&big5).into_owned();
        let inv = parse_left(&mangled, Some(&big5)).unwrap();
        assert_eq!(inv.items[0].name, "茶");
    }

    #[test]
    fn fixed_part_only_is_accepted() {
        let inv = parse_left(&fixed(), None).unwrap();
        assert!(inv.items.is_empty());
        assert_eq!(inv.encoding, None);
    }

    #[test]
    fn rejects_lookalikes() {
        assert!(parse_left("https://example.com", None).is_none());
        let bad_number = fixed().replacen("AB", "ab", 1);
        assert!(parse_left(&bad_number, None).is_none());
        let bad_date = fixed().replacen("1151005", "1151305", 1);
        assert!(parse_left(&bad_date, None).is_none());
    }
}
