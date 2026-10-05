//! One-click Wi-Fi connection from a `WIFI:` QR code.
//!
//! * Windows: a temporary WLAN profile is added with `netsh wlan add profile` and the file is
//!   deleted immediately afterwards (it contains the password).
//! * Linux: `nmcli device wifi connect` (NetworkManager).
//! * macOS: `networksetup -setairportnetwork` on the Wi-Fi interface.
//!
//! All values are passed as separate process arguments — never through a shell — and the
//! Windows profile XML is escaped, so a malicious SSID cannot inject commands.

use std::process::Command;

use qrtool_core::wifi::{WifiConfig, WifiSecurity};

/// Wi-Fi connection errors.
#[derive(Debug, thiserror::Error)]
pub enum WifiError {
    #[error("connecting to Wi-Fi is not supported on this system")]
    Unsupported,
    #[error("the system tool failed: {0}")]
    Tool(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// `true` if [`connect`] is implemented for this platform.
pub fn is_supported() -> bool {
    cfg!(any(windows, target_os = "linux", target_os = "macos"))
}

/// Connects to the network described by `cfg`.
pub fn connect(cfg: &WifiConfig) -> Result<(), WifiError> {
    #[cfg(windows)]
    {
        connect_windows(cfg)
    }
    #[cfg(target_os = "linux")]
    {
        connect_linux(cfg)
    }
    #[cfg(target_os = "macos")]
    {
        connect_macos(cfg)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = cfg;
        Err(WifiError::Unsupported)
    }
}

fn run(cmd: &mut Command) -> Result<String, WifiError> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        Ok(text)
    } else {
        Err(WifiError::Tool(text.trim().to_string()))
    }
}

/// Escapes text for inclusion in XML element content.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

fn hex(s: &str) -> String {
    s.as_bytes().iter().map(|b| format!("{b:02X}")).collect()
}

/// Builds a Windows WLAN profile XML. The profile name is derived from the SSID's hex so it
/// never contains spaces or quotes (important for `netsh` argument parsing).
pub fn windows_profile_xml(cfg: &WifiConfig) -> (String, String) {
    let profile_name = format!("QRcodeTool-{}", &hex(&cfg.ssid)[..hex(&cfg.ssid).len().min(48)]);
    let (auth, enc, key) = match &cfg.security {
        WifiSecurity::Open => ("open", "none", None),
        WifiSecurity::Wep => ("open", "WEP", cfg.password.as_deref().map(|p| ("networkKey", p))),
        WifiSecurity::Wpa3 => (
            "WPA3SAE",
            "AES",
            cfg.password.as_deref().map(|p| ("passPhrase", p)),
        ),
        WifiSecurity::Wpa | WifiSecurity::Other(_) => (
            "WPA2PSK",
            "AES",
            cfg.password.as_deref().map(|p| ("passPhrase", p)),
        ),
    };
    let shared_key = key
        .map(|(kind, pw)| {
            format!(
                "<sharedKey><keyType>{kind}</keyType><protected>false</protected><keyMaterial>{}</keyMaterial></sharedKey>",
                xml_escape(pw)
            )
        })
        .unwrap_or_default();
    let xml = format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
  <name>{name}</name>
  <SSIDConfig><SSID><hex>{hex}</hex><name>{ssid}</name></SSID><nonBroadcast>{hidden}</nonBroadcast></SSIDConfig>
  <connectionType>ESS</connectionType>
  <connectionMode>manual</connectionMode>
  <MSM><security><authEncryption><authentication>{auth}</authentication><encryption>{enc}</encryption><useOneX>false</useOneX></authEncryption>{shared_key}</security></MSM>
</WLANProfile>
"#,
        name = xml_escape(&profile_name),
        hex = hex(&cfg.ssid),
        ssid = xml_escape(&cfg.ssid),
        hidden = cfg.hidden,
    );
    (profile_name, xml)
}

#[cfg(windows)]
fn connect_windows(cfg: &WifiConfig) -> Result<(), WifiError> {
    let (profile_name, xml) = windows_profile_xml(cfg);
    let dir = std::env::temp_dir();
    let file_name = format!("qrcode-tool-wlan-{}.xml", std::process::id());
    let path = dir.join(&file_name);
    std::fs::write(&path, xml)?;
    // Run inside the temp dir and pass a relative file name: avoids quoting problems with
    // spaces in the user's profile path.
    let add = run(Command::new("netsh")
        .current_dir(&dir)
        .args(["wlan", "add", "profile"])
        .arg(format!("filename={file_name}"))
        .arg("user=current"));
    let _ = std::fs::remove_file(&path);
    add?;
    run(Command::new("netsh")
        .args(["wlan", "connect"])
        .arg(format!("name={profile_name}")))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn connect_linux(cfg: &WifiConfig) -> Result<(), WifiError> {
    let mut cmd = Command::new("nmcli");
    cmd.args(["device", "wifi", "connect"]).arg(&cfg.ssid);
    if let Some(pw) = &cfg.password {
        cmd.arg("password").arg(pw);
    }
    if cfg.hidden {
        cmd.args(["hidden", "yes"]);
    }
    run(&mut cmd).map(|_| ())
}

#[cfg(target_os = "macos")]
fn connect_macos(cfg: &WifiConfig) -> Result<(), WifiError> {
    let ports = run(Command::new("networksetup").arg("-listallhardwareports"))?;
    let device = wifi_device_from_ports(&ports).ok_or(WifiError::Unsupported)?;
    let mut cmd = Command::new("networksetup");
    cmd.arg("-setairportnetwork").arg(device).arg(&cfg.ssid);
    if let Some(pw) = &cfg.password {
        cmd.arg(pw);
    }
    run(&mut cmd).map(|_| ())
}

/// Finds the Wi-Fi device name (e.g. `en0`) in `networksetup -listallhardwareports` output.
pub fn wifi_device_from_ports(output: &str) -> Option<String> {
    let mut lines = output.lines();
    while let Some(l) = lines.next() {
        let port = l.trim();
        if port.starts_with("Hardware Port:") && (port.contains("Wi-Fi") || port.contains("AirPort")) {
            for next in lines.by_ref() {
                if let Some(dev) = next.trim().strip_prefix("Device:") {
                    return Some(dev.trim().to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(ssid: &str, pw: Option<&str>, sec: WifiSecurity) -> WifiConfig {
        WifiConfig {
            ssid: ssid.into(),
            password: pw.map(Into::into),
            security: sec,
            hidden: false,
        }
    }

    #[test]
    fn profile_xml_is_escaped() {
        let (name, xml) = windows_profile_xml(&cfg(r#"Evil"</name><x>&"#, Some("p<&>w"), WifiSecurity::Wpa));
        assert!(!name.contains(' ') && !name.contains('"'));
        assert!(xml.contains("Evil&quot;&lt;/name&gt;&lt;x&gt;&amp;"));
        assert!(xml.contains("<keyMaterial>p&lt;&amp;&gt;w</keyMaterial>"));
        assert!(xml.contains("<authentication>WPA2PSK</authentication>"));
    }

    #[test]
    fn open_network_has_no_key() {
        let (_, xml) = windows_profile_xml(&cfg("Guest", None, WifiSecurity::Open));
        assert!(!xml.contains("sharedKey"));
        assert!(xml.contains("<encryption>none</encryption>"));
    }

    #[test]
    fn finds_macos_wifi_device() {
        let out = "Hardware Port: Ethernet\nDevice: en1\n\nHardware Port: Wi-Fi\nDevice: en0\nEthernet Address: x\n";
        assert_eq!(wifi_device_from_ports(out).as_deref(), Some("en0"));
        assert_eq!(wifi_device_from_ports("nothing"), None);
    }
}
