use crate::domain::WindowAction;
use crate::ports::WindowManager;

pub struct MacOSWindowManager;

impl WindowManager for MacOSWindowManager {
    fn perform_action(&self, action: WindowAction) {
        log::debug!("Performing window action: {:?}", action);
        let frame_script = match action {
            WindowAction::LeftHalf => "{0, top_inset, desktop_width / 2, usable_height}",
            WindowAction::RightHalf => {
                "{desktop_width / 2, top_inset, desktop_width / 2, usable_height}"
            }
            WindowAction::TopHalf => "{0, top_inset, desktop_width, usable_height / 2}",
            WindowAction::BottomHalf => {
                "{0, top_inset + usable_height / 2, desktop_width, usable_height / 2}"
            }

            WindowAction::TopLeft => "{0, top_inset, desktop_width / 2, usable_height / 2}",
            WindowAction::TopRight => {
                "{desktop_width / 2, top_inset, desktop_width / 2, usable_height / 2}"
            }
            WindowAction::BottomLeft => {
                "{0, top_inset + usable_height / 2, desktop_width / 2, usable_height / 2}"
            }
            WindowAction::BottomRight => {
                "{desktop_width / 2, top_inset + usable_height / 2, desktop_width / 2, usable_height / 2}"
            }

            WindowAction::TopLeftSixth => {
                "{0, top_inset, desktop_width / 3, usable_height / 2}"
            }
            WindowAction::TopCenterSixth => {
                "{desktop_width / 3, top_inset, desktop_width / 3, usable_height / 2}"
            }
            WindowAction::TopRightSixth => {
                "{desktop_width * 2 / 3, top_inset, desktop_width / 3, usable_height / 2}"
            }
            WindowAction::BottomLeftSixth => {
                "{0, top_inset + usable_height / 2, desktop_width / 3, usable_height / 2}"
            }
            WindowAction::BottomCenterSixth => {
                "{desktop_width / 3, top_inset + usable_height / 2, desktop_width / 3, usable_height / 2}"
            }
            WindowAction::BottomRightSixth => {
                "{desktop_width * 2 / 3, top_inset + usable_height / 2, desktop_width / 3, usable_height / 2}"
            }

            WindowAction::Maximize => "{0, top_inset, desktop_width, usable_height}",
            WindowAction::Center => {
                "{desktop_width / 4, top_inset + usable_height / 4, desktop_width / 2, usable_height / 2}"
            }
        };

        let script = format!(
            r#"
            tell application "Finder"
                set desktop_bounds to bounds of window of desktop
                set desktop_width to item 3 of desktop_bounds
                set desktop_height to item 4 of desktop_bounds
                set top_inset to 23
                set usable_height to desktop_height - top_inset
            end tell
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set frontWindow to a reference to window 1 of frontApp
                set target_frame to {}
                set position of frontWindow to {{item 1 of target_frame, item 2 of target_frame}}
                set size of frontWindow to {{item 3 of target_frame, item 4 of target_frame}}
            end tell
        "#,
            frame_script
        );

        match std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
        {
            Ok(output) if !output.status.success() => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::error!("Window action AppleScript failed: {}", stderr.trim());
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to run osascript for window action: {}", e);
            }
        }
    }
}
