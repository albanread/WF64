//! Frame-level "Tools" menu and the keyboard accelerator table for
//! fedit's built-in tool windows.
//!
//! Both `fedit` and `log_view` are always-available editor tools
//! that hang off a `Tools` submenu on the frame. Keeping their
//! menu/accelerator wiring together here means the one-and-only
//! Tools popup carries every entry, regardless of whether the
//! language thread has installed a custom menu.

#![cfg(windows)]

use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateAcceleratorTableW, CreateMenu, CreatePopupMenu, ACCEL, FCONTROL, FSHIFT,
    FVIRTKEY, HACCEL, HMENU, MF_POPUP, MF_STRING,
};

use super::log_view;
use super::fedit;
use super::fconsole;
use super::crash_view;
use windows::Win32::UI::WindowsAndMessaging::MF_SEPARATOR;

/// Append an `Edit` submenu to `bar`.  Routed to the active MDI
/// child via WM_COMMAND forwarding from the frame WndProc; fedit
/// recognises the IDs in its own WM_COMMAND handler and dispatches
/// to the matching method.
///
/// Forth-shaped menu: the sexp ops the Lisp version exposed
/// (Forward S-expression / Slurp / Barf / Wrap / Splice / Raise)
/// are gone — replaced by simple word-boundary navigation that
/// makes sense for whitespace-delimited Forth tokens.  Run-Form
/// is also gone (no single "form" to run in Forth); Run-Buffer
/// is kept as F5 → evaluate the whole buffer.
pub fn append_edit_menu(bar: HMENU) {
    let popup = match unsafe { CreatePopupMenu() } {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[edit-menu] CreatePopupMenu failed: {e}");
            return;
        }
    };

    let items: &[(u16, &str)] = &[
        (fedit::EDIT_CMD_UNDO, "&Undo\tCtrl+Z"),
        (fedit::EDIT_CMD_REDO, "&Redo\tCtrl+Y"),
        (0, "SEP"),
        (fedit::EDIT_CMD_CUT, "Cu&t\tCtrl+X"),
        (fedit::EDIT_CMD_COPY, "&Copy\tCtrl+C"),
        (fedit::EDIT_CMD_PASTE, "&Paste\tCtrl+V"),
        (fedit::EDIT_CMD_SELECT_ALL, "Select &All\tCtrl+A"),
        (0, "SEP"),
        (fedit::EDIT_CMD_NEXT_WORD, "Next &Word\tCtrl+\u{2192}"),
        (fedit::EDIT_CMD_PREV_WORD, "Pre&v Word\tCtrl+\u{2190}"),
        (0, "SEP"),
        (fedit::EDIT_CMD_RUN_BUFFER, "R&un Buffer\tF5"),
    ];

    for &(id, label) in items {
        if label == "SEP" {
            let _ = unsafe { AppendMenuW(popup, MF_SEPARATOR, 0, PCWSTR::null()) };
            continue;
        }
        let mut w: Vec<u16> = label.encode_utf16().collect();
        w.push(0);
        if let Err(e) = unsafe {
            AppendMenuW(popup, MF_STRING, id as usize, PCWSTR(w.as_ptr()))
        } {
            eprintln!("[edit-menu] append {label:?}: {e}");
        }
    }

    let title: Vec<u16> = "&Edit\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(bar, MF_POPUP, popup.0 as usize, PCWSTR(title.as_ptr()))
    } {
        eprintln!("[edit-menu] append popup: {e}");
    }
}

/// Append a `Tools` submenu to `bar` containing every built-in tool
/// (currently fedit and the log view). Called both from
/// `build_default_menu_bar` and from `menu::install_for_frame` so
/// the tools stay reachable whatever the language thread does.
pub fn append_tools_menu(bar: HMENU) {
    let popup = match unsafe { CreatePopupMenu() } {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[tools-menu] CreatePopupMenu failed: {e}");
            return;
        }
    };

    let fedit_item: Vec<u16> = "fedit\tCtrl+Shift+E\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            popup,
            MF_STRING,
            fedit::MENU_CMD_ID as usize,
            PCWSTR(fedit_item.as_ptr()),
        )
    } {
        eprintln!("[tools-menu] append fedit: {e}");
    }

    let log_item: Vec<u16> = "Log\tCtrl+Shift+L\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            popup,
            MF_STRING,
            log_view::MENU_CMD_ID as usize,
            PCWSTR(log_item.as_ptr()),
        )
    } {
        eprintln!("[tools-menu] append log: {e}");
    }

    let console_item: Vec<u16> = "Console\tCtrl+Shift+R\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            popup,
            MF_STRING,
            fconsole::MENU_CMD_ID as usize,
            PCWSTR(console_item.as_ptr()),
        )
    } {
        eprintln!("[tools-menu] append console: {e}");
    }

    let crash_item: Vec<u16> = "Crash dump\tCtrl+Shift+X\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            popup,
            MF_STRING,
            crash_view::MENU_CMD_ID as usize,
            PCWSTR(crash_item.as_ptr()),
        )
    } {
        eprintln!("[tools-menu] append crash: {e}");
    }

    let title: Vec<u16> = "&Tools\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            bar,
            MF_POPUP,
            popup.0 as usize,
            PCWSTR(title.as_ptr()),
        )
    } {
        eprintln!("[tools-menu] append popup: {e}");
    }
}

/// Frame-level WM_COMMAND id for the Forth-Restart menu item.
/// Living in tools_menu so all menu IDs sit together.
pub const FORTH_RESTART_CMD_ID: u16 = 0x3200;

/// Append a `Forth` submenu to `bar` carrying the language-thread
/// lifecycle commands.  Currently just Restart; Reload-core and
/// Trace toggles will join later.
pub fn append_forth_menu(bar: HMENU) {
    let popup = match unsafe { CreatePopupMenu() } {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[forth-menu] CreatePopupMenu failed: {e}");
            return;
        }
    };

    let restart_item: Vec<u16> = "&Restart\tCtrl+Shift+F5\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(
            popup,
            MF_STRING,
            FORTH_RESTART_CMD_ID as usize,
            PCWSTR(restart_item.as_ptr()),
        )
    } {
        eprintln!("[forth-menu] append restart: {e}");
    }

    let title: Vec<u16> = "&Forth\0".encode_utf16().collect();
    if let Err(e) = unsafe {
        AppendMenuW(bar, MF_POPUP, popup.0 as usize, PCWSTR(title.as_ptr()))
    } {
        eprintln!("[forth-menu] append popup: {e}");
    }
}

/// Build a stand-alone menu bar containing the Edit, Forth, and
/// Tools submenus.  Used at frame startup when no language-thread
/// menu has been set.
pub fn build_default_menu_bar() -> Option<HMENU> {
    let bar = unsafe { CreateMenu() }.ok()?;
    append_edit_menu(bar);
    append_forth_menu(bar);
    append_tools_menu(bar);
    Some(bar)
}

/// Frame-level accelerator table:
///   Ctrl+Shift+E → fedit
///   Ctrl+Shift+L → log view
/// Both dispatch via `WM_COMMAND` to their respective MENU_CMD_IDs,
/// which the frame WndProc routes to the right `open` function.
pub fn build_accelerator_table() -> Option<HACCEL> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_F5;
    let entries = [
        ACCEL {
            fVirt: FCONTROL | FSHIFT | FVIRTKEY,
            key: b'E' as u16,
            cmd: fedit::MENU_CMD_ID,
        },
        ACCEL {
            fVirt: FCONTROL | FSHIFT | FVIRTKEY,
            key: b'L' as u16,
            cmd: log_view::MENU_CMD_ID,
        },
        ACCEL {
            fVirt: FCONTROL | FSHIFT | FVIRTKEY,
            key: b'R' as u16,
            cmd: fconsole::MENU_CMD_ID,
        },
        ACCEL {
            fVirt: FCONTROL | FSHIFT | FVIRTKEY,
            key: VK_F5.0,
            cmd: FORTH_RESTART_CMD_ID,
        },
        ACCEL {
            fVirt: FCONTROL | FSHIFT | FVIRTKEY,
            key: b'X' as u16,
            cmd: crash_view::MENU_CMD_ID,
        },
    ];
    unsafe { CreateAcceleratorTableW(&entries) }
        .ok()
        .filter(|h| !h.is_invalid())
}
