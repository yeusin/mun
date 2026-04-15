use crate::domain::WindowAction;
use crate::ports::WindowManager;

pub struct MacOSWindowManager;

impl WindowManager for MacOSWindowManager {
    fn perform_action(&self, action: WindowAction) {
        let bounds_script = match action {
            WindowAction::LeftHalf => "{0, 23, desktop_width / 2, desktop_height}",
            WindowAction::RightHalf => "{desktop_width / 2, 23, desktop_width, desktop_height}",
            WindowAction::TopHalf => "{0, 23, desktop_width, desktop_height / 2}",
            WindowAction::BottomHalf => "{0, desktop_height / 2, desktop_width, desktop_height}",

            WindowAction::TopLeft => "{0, 23, desktop_width / 2, desktop_height / 2}",
            WindowAction::TopRight => "{desktop_width / 2, 23, desktop_width, desktop_height / 2}",
            WindowAction::BottomLeft => {
                "{0, desktop_height / 2, desktop_width / 2, desktop_height}"
            }
            WindowAction::BottomRight => {
                "{desktop_width / 2, desktop_height / 2, desktop_width, desktop_height}"
            }

            WindowAction::TopLeftSixth => "{0, 23, desktop_width / 3, desktop_height / 2}",
            WindowAction::TopCenterSixth => {
                "{desktop_width / 3, 23, desktop_width * 2 / 3, desktop_height / 2}"
            }
            WindowAction::TopRightSixth => {
                "{desktop_width * 2 / 3, 23, desktop_width, desktop_height / 2}"
            }
            WindowAction::BottomLeftSixth => {
                "{0, desktop_height / 2, desktop_width / 3, desktop_height}"
            }
            WindowAction::BottomCenterSixth => {
                "{desktop_width / 3, desktop_height / 2, desktop_width * 2 / 3, desktop_height}"
            }
            WindowAction::BottomRightSixth => {
                "{desktop_width * 2 / 3, desktop_height / 2, desktop_width, desktop_height}"
            }

            WindowAction::Maximize => "{0, 23, desktop_width, desktop_height}",
            WindowAction::Center => {
                "{desktop_width / 4, desktop_height / 4, desktop_width * 3 / 4, desktop_height * 3 / 4}"
            }
        };

        let script = format!(
            r#"
            tell application "Finder"
                set desktop_bounds to bounds of window of desktop
                set desktop_width to item 3 of desktop_bounds
                set desktop_height to item 4 of desktop_bounds
            end tell
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set frontWindow to window 1 of frontApp
                set bounds of frontWindow to {}
            end tell
        "#,
            bounds_script
        );

        if let Err(e) = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
        {
            log::error!("Failed to run osascript for window action: {}", e);
        }
    }
}
