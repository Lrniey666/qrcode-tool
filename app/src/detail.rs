//! The detail card: what is shown for each kind of code and what its buttons do.

use std::rc::Rc;

use qrtool_core::payload::Payload;
use qrtool_core::url_safety::{self, Severity};
use qrtool_core::wifi::WifiSecurity;
use slint::{ModelRc, VecModel};

use crate::scan::{display, format_label, ScanItem};
use crate::ui;

/// Builds the card for `item`. `confirm_danger` is set after the first "Open" press on
/// a dangerous link: the card then offers "Open anyway".
pub fn build(
    item: &ScanItem,
    confirm_danger: bool,
    trusted: &[String],
    s: &ui::Strings<'_>,
) -> ui::DetailData {
    let mut fields: Vec<ui::FieldRow> = Vec::new();
    let mut findings: Vec<ui::FindingRow> = Vec::new();
    let mut actions: Vec<ui::ActionRow> = Vec::new();
    let mut can_trust = false;
    let mut trust_domain = String::new();

    let field = |key: &str, value: &str| ui::FieldRow {
        key: key.into(),
        value: display(value, 4000).into(),
        copyable: true,
        secret: false,
    };
    let action = |id: &str, primary: bool, danger: bool| ui::ActionRow {
        id: id.into(),
        primary,
        danger,
    };

    let title = match &item.payload {
        Payload::Url(r) => {
            fields.push(field("url", &r.url));
            if r.host_ascii != r.host_display {
                fields.push(field("host", &r.host_ascii));
            }
            for f in &r.findings {
                findings.push(ui::FindingRow {
                    code: f.code().into(),
                    detail: display(&f.detail(), 80).into(),
                    level: f.severity() as i32,
                });
            }
            if r.level >= Severity::Danger {
                if confirm_danger {
                    actions.push(action("open-anyway", false, true));
                } else {
                    actions.push(action("open", false, false));
                }
            } else {
                actions.push(action("open", true, false));
            }
            actions.push(action("copy", false, false));
            if r.level < Severity::Warning && !url_safety::is_trusted(&r.host_ascii, trusted) {
                can_trust = true;
                trust_domain = r.host_key.clone();
            }
            display(&r.host_display, 80)
        }
        Payload::OtherScheme { scheme } => {
            fields.push(field("scheme", scheme));
            fields.push(field("content", &item.text));
            actions.push(action("copy", true, false));
            display(&item.text, 80)
        }
        Payload::Wifi(w) => {
            fields.push(field("ssid", &w.ssid));
            let security = match w.security {
                WifiSecurity::Open => s.invoke_t("open-network".into()).to_string(),
                ref other => other.label(),
            };
            fields.push(ui::FieldRow {
                copyable: false,
                ..field("security", &security)
            });
            if let Some(pw) = &w.password {
                fields.push(ui::FieldRow {
                    key: "password".into(),
                    value: pw.as_str().into(),
                    copyable: true,
                    secret: true,
                });
            }
            if w.hidden {
                fields.push(ui::FieldRow {
                    copyable: false,
                    ..field("hidden", &s.invoke_t("yes".into()))
                });
            }
            if qrtool_platform::wifi::is_supported() {
                actions.push(action("connect-wifi", true, false));
            }
            if w.password.is_some() {
                actions.push(action(
                    "copy-password",
                    !qrtool_platform::wifi::is_supported(),
                    false,
                ));
            }
            display(&w.ssid, 80)
        }
        Payload::Contact(c) => {
            if !c.name.is_empty() {
                fields.push(field("name", &c.name));
            }
            if let Some(org) = &c.organization {
                fields.push(field("organization", org));
            }
            for p in &c.phones {
                fields.push(field("phone", p));
            }
            for e in &c.emails {
                fields.push(field("email", e));
            }
            for u in &c.urls {
                fields.push(field("website", u));
            }
            if let Some(a) = &c.address {
                fields.push(field("postal", a));
            }
            if let Some(n) = &c.note {
                fields.push(field("note", n));
            }
            actions.push(action("save-contact", true, false));
            actions.push(action("copy", false, false));
            display(&c.name, 80)
        }
        Payload::Email { to, subject, body } => {
            fields.push(field("email", to));
            if let Some(sub) = subject {
                fields.push(field("subject", sub));
            }
            if let Some(b) = body {
                fields.push(field("body", b));
            }
            actions.push(action("send-email", true, false));
            actions.push(action("copy", false, false));
            display(to, 80)
        }
        Payload::Phone { number } => {
            fields.push(field("phone", number));
            actions.push(action("call", true, false));
            actions.push(action("copy", false, false));
            display(number, 80)
        }
        Payload::Sms { number, body } => {
            fields.push(field("phone", number));
            if let Some(b) = body {
                fields.push(field("body", b));
            }
            actions.push(action("send-sms", true, false));
            actions.push(action("copy", false, false));
            display(number, 80)
        }
        Payload::Geo { lat, lon, query } => {
            fields.push(field("coords", &format!("{lat:.6}, {lon:.6}")));
            if let Some(q) = query {
                fields.push(field("content", q));
            }
            actions.push(action("open-map", true, false));
            actions.push(action("copy", false, false));
            format!("{lat:.5}, {lon:.5}")
        }
        Payload::Otp { issuer, account } => {
            if let Some(i) = issuer {
                fields.push(field("issuer", i));
            }
            if let Some(a) = account {
                fields.push(field("account", a));
            }
            actions.push(action("copy", false, false));
            display(issuer.as_deref().or(account.as_deref()).unwrap_or_default(), 80)
        }
        Payload::Crypto {
            scheme,
            address,
            amount,
        } => {
            fields.push(field("scheme", scheme));
            fields.push(field("wallet", address));
            if let Some(a) = amount {
                fields.push(field("amount", a));
            }
            actions.push(action("copy", true, false));
            display(address, 48)
        }
        Payload::EInvoice(inv) => {
            fields.push(field("invoice-number", &inv.number));
            fields.push(field("date", &inv.date));
            fields.push(field("random-code", &inv.random_code));
            fields.push(field("total", &format!("NT$ {}", inv.total_amount)));
            fields.push(ui::FieldRow {
                copyable: false,
                ..field("tax", &format!("NT$ {}", inv.tax_amount()))
            });
            fields.push(field("seller", &inv.seller_id));
            if let Some(b) = &inv.buyer_id {
                fields.push(field("buyer", b));
            }
            if !inv.items.is_empty() {
                let lines: Vec<String> = inv
                    .items
                    .iter()
                    .map(|it| format!("{} × {} @ {}", it.name, it.quantity, it.unit_price))
                    .collect();
                fields.push(ui::FieldRow {
                    value: lines.join("\n").into(),
                    ..field("items", "")
                });
            }
            let missing = match (inv.items_in_code, inv.total_items) {
                (Some(a), Some(b)) => (inv.items.len() as u32) < b.max(a),
                _ => false,
            };
            if missing && !inv.has_right_part {
                fields.push(ui::FieldRow {
                    copyable: false,
                    ..field("note", &s.invoke_t("einvoice-items-hint".into()))
                });
            }
            actions.push(action("copy-number", true, false));
            actions.push(action("copy", false, false));
            inv.number.clone()
        }
        Payload::EInvoiceContinuation => {
            fields.push(field("content", &item.text));
            actions.push(action("copy", true, false));
            s.invoke_kind_label("einvoice-cont".into()).to_string()
        }
        Payload::Text => {
            fields.push(field("content", &item.text));
            if !item.is_qr() {
                fields.push(ui::FieldRow {
                    copyable: false,
                    ..field("format", &format_label(&item.format))
                });
            }
            actions.push(action("copy", true, false));
            display(&item.text, 80)
        }
    };

    actions.push(action("close", false, false));

    ui::DetailData {
        kind: item.payload.kind().id().into(),
        title: title.into(),
        content: display(&item.text, 4000).into(),
        level: item.level(),
        fields: ModelRc::from(Rc::new(VecModel::from(fields))),
        findings: ModelRc::from(Rc::new(VecModel::from(findings))),
        actions: ModelRc::from(Rc::new(VecModel::from(actions))),
        confirm_danger,
        can_trust,
        trust_domain: trust_domain.into(),
    }
}

/// What the caller should do after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The code was used (opened, connected…): close the card and usually the scan.
    Done,
    /// Something was copied: keep the scan open, close the card.
    Copied,
    /// Rebuild the card (e.g. dangerous link after the first "Open" press, site trusted).
    Rebuild { confirm_danger: bool },
    /// Close only the card.
    CloseCard,
    /// Show an error message (kind, argument).
    Error(&'static str, String),
    /// Nothing to do yet (the action finishes asynchronously).
    Pending,
}

/// Runs a detail-card action.
pub fn perform(action: &str, item: &ScanItem, confirm_danger: bool, s: &ui::Strings<'_>) -> Outcome {
    use qrtool_platform::clipboard;
    match action {
        "open" | "open-anyway" | "send-email" | "call" | "send-sms" | "open-map" => {
            if let Payload::Url(r) = &item.payload {
                if r.level >= Severity::Danger && action == "open" && !confirm_danger {
                    return Outcome::Rebuild { confirm_danger: true };
                }
            }
            match crate::scan::open_item(item) {
                Ok(()) => Outcome::Done,
                Err(e) => Outcome::Error("open-failed", e),
            }
        }
        "copy" => {
            let text = match &item.payload {
                Payload::Url(r) => r.url.clone(),
                _ => item.text.clone(),
            };
            clipboard::set_text(&text);
            Outcome::Copied
        }
        "copy-password" => {
            if let Payload::Wifi(w) = &item.payload {
                if let Some(pw) = &w.password {
                    clipboard::set_text(pw);
                }
            }
            Outcome::Copied
        }
        "copy-number" => {
            if let Payload::EInvoice(inv) = &item.payload {
                clipboard::set_text(&inv.number);
            }
            Outcome::Copied
        }
        "connect-wifi" => {
            if let Payload::Wifi(w) = &item.payload {
                let cfg = w.clone();
                std::thread::spawn(move || {
                    let res = qrtool_platform::wifi::connect(&cfg);
                    let ssid = cfg.ssid.clone();
                    let _ = slint::invoke_from_event_loop(move || match res {
                        Ok(()) => crate::message::show("wifi-connected", &display(&ssid, 64), 0),
                        Err(e) => crate::message::show("wifi-failed", &e.to_string(), 2),
                    });
                });
                return Outcome::Done;
            }
            Outcome::CloseCard
        }
        "save-contact" => {
            if let Payload::Contact(c) = &item.payload {
                let vcard = if item
                    .text
                    .trim_start()
                    .to_ascii_uppercase()
                    .starts_with("BEGIN:VCARD")
                {
                    item.text.clone()
                } else {
                    c.to_vcard()
                };
                let name = sanitize_file_name(if c.name.is_empty() { "contact" } else { &c.name });
                let title = s.invoke_t("dialog-save-contact".into()).to_string();
                let filter = s.invoke_t("filter-contact".into()).to_string();
                let _ = slint::spawn_local(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title(title)
                        .add_filter(filter, &["vcf"])
                        .set_file_name(format!("{name}.vcf"))
                        .save_file()
                        .await;
                    if let Some(file) = file {
                        if let Err(e) = std::fs::write(file.path(), vcard) {
                            crate::message::show("open-failed", &e.to_string(), 3);
                        }
                    }
                });
                return Outcome::Pending;
            }
            Outcome::CloseCard
        }
        "trust" => {
            if let Payload::Url(r) = &item.payload {
                let domain = url_safety::normalize_trusted_entry(&r.host_key);
                crate::app::app().update_settings(|st| {
                    if !st.scan.trusted_domains.contains(&domain) {
                        st.scan.trusted_domains.push(domain.clone());
                        st.scan.trusted_domains.sort();
                    }
                });
            }
            Outcome::Rebuild { confirm_danger }
        }
        _ => Outcome::CloseCard,
    }
}

/// Characters that are not allowed in file names on any platform are replaced.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "contact".into()
    } else {
        qrtool_core::text::truncate_chars(trimmed, 60)
            .trim_end_matches('…')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_file_name;

    #[test]
    fn file_names_are_safe() {
        assert_eq!(sanitize_file_name("A/B:C"), "A_B_C");
        assert_eq!(sanitize_file_name(".."), "contact");
        assert_eq!(sanitize_file_name("王小明"), "王小明");
    }
}
