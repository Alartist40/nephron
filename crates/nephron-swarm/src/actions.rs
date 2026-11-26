//! CUA-style OS-level computer control actions for Nephron Swarm subagents.

use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum ComputerAction {
    Notify { title: String, body: String },
    OpenUrl(String),
    OpenPath(PathBuf),
    LaunchApp(String),
    CaptureScreen(PathBuf),
    AllowlistedCheck { bin: &'static str, args: Vec<String> },
    ClickAt { x: i32, y: i32 },
    TypeText(String),
}

const ALLOWED_BINS: &[&str] = &["git", "cargo", "ls", "uname", "df", "free"];
const ALLOWED_APPS: &[&str] = &["gnome-terminal", "firefox", "chromium", "thunar", "mousepad"];

pub fn execute_action(action: &ComputerAction) -> Result<String> {
    match action {
        ComputerAction::Notify { title, body } => {
            let status = Command::new("notify-send").args([title, body]).status()?;
            if !status.success() {
                bail!("notify-send failed: {status}");
            }
            Ok(format!("Notified: {title}"))
        }
        ComputerAction::OpenUrl(url) => {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                bail!("rejected non-http(s) url: {url}");
            }
            Command::new("xdg-open").arg(url).spawn()?;
            Ok(format!("Opened {url}"))
        }
        ComputerAction::OpenPath(path) => {
            if !path.exists() {
                bail!("path does not exist: {}", path.display());
            }
            Command::new("xdg-open").arg(path).spawn()?;
            Ok(format!("Opened {}", path.display()))
        }
        ComputerAction::LaunchApp(app) => {
            if !ALLOWED_APPS.contains(&app.as_str()) {
                bail!("app not in CUA allowlist: {app}");
            }
            Command::new(app).spawn()?;
            Ok(format!("Launched {app}"))
        }
        ComputerAction::CaptureScreen(out) => {
            let ok = Command::new("gnome-screenshot")
                .arg("-f")
                .arg(out)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                || Command::new("xfce4-screenshooter")
                    .args(["-f", "-s"])
                    .arg(out)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            if !ok {
                bail!("no working screenshot tool (gnome-screenshot / xfce4-screenshooter)");
            }
            Ok(format!("Captured {}", out.display()))
        }
        ComputerAction::AllowlistedCheck { bin, args } => {
            if !ALLOWED_BINS.contains(bin) {
                bail!("binary not in allowlist: {bin}");
            }
            let out = Command::new(bin).args(args).output()?;
            let text = String::from_utf8_lossy(&out.stdout);
            Ok(text.lines().take(2).collect::<Vec<_>>().join(" | "))
        }
        ComputerAction::ClickAt { x, y } => {
            let ok = Command::new("xdotool")
                .args(["mousemove", &x.to_string(), &y.to_string(), "click", "1"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                bail!("install xdotool (X11) or ydotool (Wayland) for pointer click");
            }
            Ok(format!("Clicked ({x}, {y})"))
        }
        ComputerAction::TypeText(text) => {
            let ok = Command::new("wtype")
                .arg(text)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                || Command::new("xdotool")
                    .args(["type", "--", text])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
            if !ok {
                bail!("install wtype (Wayland) or xdotool (X11) for synthetic typing");
            }
            Ok(format!("Typed {} chars", text.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_disallowed_bin_app_and_bad_url() {
        assert!(execute_action(&ComputerAction::OpenUrl("file:///etc/passwd".into())).is_err());
        assert!(execute_action(&ComputerAction::LaunchApp("bash".into())).is_err());
        let bad = ComputerAction::AllowlistedCheck { bin: "rm", args: vec!["-rf".into()] };
        assert!(execute_action(&bad).is_err());
    }
}
