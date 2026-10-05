//! Windows desktop and image right-click menus.
//!
//! Entries live in the current user's registry (`HKCU\Software\Classes\...`), so no
//! administrator rights are needed. Each item starts this program with a fixed command.
//! A running copy receives that command and the new process exits. The image entry stores
//! only the placeholder `%1`; Explorer fills in the file path when the user clicks.
//!
//! Other platforms: [`install`] and [`remove`] do nothing.

/// Menu captions, already translated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    pub scan_screen: String,
    pub scan_region: String,
    pub scan_camera: String,
    pub toolbar: String,
    pub open_image: String,
}

/// Writes the right-click entries for `labels`. Returns `false` if a write failed.
pub fn install(labels: &Labels) -> bool {
    #[cfg(windows)]
    {
        win::install(labels)
    }
    #[cfg(not(windows))]
    {
        let _ = labels;
        true
    }
}

/// Removes every entry this module creates.
pub fn remove() -> bool {
    #[cfg(windows)]
    {
        win::remove()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// `"C:\Program Files\app.exe" scan-screen` — the program path is always quoted.
pub(crate) fn verb_command(exe: &str, args: &str) -> String {
    format!("{} {args}", quote_windows(exe))
}

fn quote_windows(arg: &str) -> String {
    // Command lines are parsed by the shell. A path from `current_exe` does not contain
    // quotes; drop any that do appear so they cannot break out of the quoted token.
    format!("\"{}\"", arg.replace('"', ""))
}

const DESKTOP_PARENT: &str = r"Software\Classes\DesktopBackground\Shell";
/// Explorer `ECF_SEPARATORBEFORE` / `ECF_SEPARATORAFTER` (`EXPCMDFLAGS`).
const SEPARATOR_BEFORE: u32 = 0x20;
const SEPARATOR_AFTER: u32 = 0x40;
const IMAGE_VERB: &str = "QRcodeTool.OpenImage";
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];
/// Explorer substitutes the selected file. The path is not written into the registry.
const SCAN_FILE_ARGS: &str = "scan-file \"%1\"";

#[cfg(windows)]
mod win {
    use super::{
        verb_command, Labels, DESKTOP_PARENT, IMAGE_EXTS, IMAGE_VERB, SCAN_FILE_ARGS, SEPARATOR_AFTER,
        SEPARATOR_BEFORE,
    };
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;

    struct Verb<'a> {
        parent: &'a str,
        id: &'a str,
        label: &'a str,
        args: &'a str,
        /// `Bottom` groups the desktop items; image items stay in the file menu.
        position: Option<&'a str>,
        single_file: bool,
        /// `ECF_SEPARATORBEFORE` / `ECF_SEPARATORAFTER` so the group is not flush
        /// against other applications' items.
        flags: u32,
    }

    pub(super) fn install(labels: &Labels) -> bool {
        let Ok(exe_path) = std::env::current_exe() else {
            tracing::warn!("right-click menu: cannot find the program path");
            return false;
        };
        let Some(exe) = exe_path.to_str() else {
            tracing::warn!("right-click menu: program path is not text");
            return false;
        };
        // The Icon value treats a comma as the icon index.
        let icon = if exe.contains(',') { "" } else { exe };

        let desktop = [
            Verb {
                parent: DESKTOP_PARENT,
                id: "QRcodeTool.1.ScanScreen",
                label: &labels.scan_screen,
                args: "scan-screen",
                position: Some("Bottom"),
                single_file: false,
                flags: SEPARATOR_BEFORE,
            },
            Verb {
                parent: DESKTOP_PARENT,
                id: "QRcodeTool.2.ScanRegion",
                label: &labels.scan_region,
                args: "scan-region",
                position: Some("Bottom"),
                single_file: false,
                flags: 0,
            },
            Verb {
                parent: DESKTOP_PARENT,
                id: "QRcodeTool.3.ScanCamera",
                label: &labels.scan_camera,
                args: "scan-camera",
                position: Some("Bottom"),
                single_file: false,
                flags: 0,
            },
            Verb {
                parent: DESKTOP_PARENT,
                id: "QRcodeTool.4.Toolbar",
                label: &labels.toolbar,
                args: "show-toolbar",
                position: Some("Bottom"),
                single_file: false,
                flags: SEPARATOR_AFTER,
            },
        ];

        let mut ok = true;
        let mut changed = false;
        for verb in &desktop {
            if is_current(verb, exe, icon) {
                continue;
            }
            changed = true;
            if let Err(err) = write_verb(verb, exe, icon) {
                tracing::warn!("right-click menu: {err}");
                ok = false;
            }
        }
        for ext in IMAGE_EXTS {
            let parent = image_parent(ext);
            let verb = Verb {
                parent: &parent,
                id: IMAGE_VERB,
                label: &labels.open_image,
                args: SCAN_FILE_ARGS,
                position: None,
                single_file: true,
                flags: SEPARATOR_BEFORE | SEPARATOR_AFTER,
            };
            if is_current(&verb, exe, icon) {
                continue;
            }
            changed = true;
            if let Err(err) = write_verb(&verb, exe, icon) {
                tracing::warn!("right-click menu: {err}");
                ok = false;
            }
        }
        if changed {
            notify_assoc_changed();
        }
        ok
    }

    pub(super) fn remove() -> bool {
        let mut changed = false;
        for id in [
            "QRcodeTool.1.ScanScreen",
            "QRcodeTool.2.ScanRegion",
            "QRcodeTool.3.ScanCamera",
            "QRcodeTool.4.Toolbar",
        ] {
            changed |= delete_verb(DESKTOP_PARENT, id);
        }
        for ext in IMAGE_EXTS {
            changed |= delete_verb(&image_parent(ext), IMAGE_VERB);
        }
        if changed {
            notify_assoc_changed();
        }
        true
    }

    fn image_parent(ext: &str) -> String {
        format!(r"Software\Classes\SystemFileAssociations\.{ext}\shell")
    }

    fn is_current(verb: &Verb<'_>, exe: &str, icon: &str) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(key) = hkcu.open_subkey(format!(r"{}\{}", verb.parent, verb.id)) else {
            return false;
        };
        let label: String = key.get_value("").unwrap_or_default();
        let current_icon: String = key.get_value("Icon").unwrap_or_default();
        let position: String = key.get_value("Position").unwrap_or_default();
        let multi: String = key.get_value("MultiSelectModel").unwrap_or_default();
        let flags: u32 = key.get_value("CommandFlags").unwrap_or(0);
        let Ok(command_key) = hkcu.open_subkey(format!(r"{}\{}\command", verb.parent, verb.id)) else {
            return false;
        };
        let command: String = command_key.get_value("").unwrap_or_default();
        label == verb.label
            && current_icon == icon
            && position == verb.position.unwrap_or("")
            && multi == if verb.single_file { "Single" } else { "" }
            && flags == verb.flags
            && command == verb_command(exe, verb.args)
    }

    fn write_verb(verb: &Verb<'_>, exe: &str, icon: &str) -> std::io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(format!(r"{}\{}", verb.parent, verb.id))?;
        key.set_value("", &verb.label)?;
        if !icon.is_empty() {
            key.set_value("Icon", &icon)?;
        }
        if let Some(position) = verb.position {
            key.set_value("Position", &position)?;
        }
        if verb.single_file {
            key.set_value("MultiSelectModel", &"Single")?;
        }
        if verb.flags == 0 {
            let _ = key.delete_value("CommandFlags");
        } else {
            key.set_value("CommandFlags", &verb.flags)?;
        }
        let (command, _) = hkcu.create_subkey(format!(r"{}\{}\command", verb.parent, verb.id))?;
        command.set_value("", &verb_command(exe, verb.args))?;
        Ok(())
    }

    fn delete_verb(parent: &str, id: &str) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(key) = hkcu.open_subkey_with_flags(parent, KEY_WRITE) else {
            return false;
        };
        key.delete_subkey_all(id).is_ok()
    }

    fn notify_assoc_changed() {
        use windows_sys::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
        // SAFETY: both item pointers are null, which SHCNF_IDLIST allows for ASSOCCHANGED.
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED as i32,
                SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_the_program_and_keeps_the_file_placeholder() {
        let cmd = verb_command(r"C:\Program Files\QRcode tool\qrcode-tool.exe", SCAN_FILE_ARGS);
        assert_eq!(
            cmd,
            r#""C:\Program Files\QRcode tool\qrcode-tool.exe" scan-file "%1""#
        );
    }

    #[test]
    fn screen_command_has_no_file_placeholder() {
        let cmd = verb_command(r"C:\qrcode-tool.exe", "scan-screen");
        assert_eq!(cmd, r#""C:\qrcode-tool.exe" scan-screen"#);
        assert!(!cmd.contains("%1"));
    }

    #[test]
    fn a_quote_in_the_path_cannot_close_the_token() {
        let cmd = verb_command(r"C:\bad\quote.exe", "scan-region");
        assert_eq!(cmd, r#""C:\bad\quote.exe" scan-region"#);
        let cmd = verb_command("C:\\a\"b.exe", "show-toolbar");
        assert_eq!(cmd, r#""C:\ab.exe" show-toolbar"#);
        assert!(!cmd.contains("\"b"));
    }
}
