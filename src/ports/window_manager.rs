use crate::domain::WindowAction;

pub trait WindowManager: Send + Sync + 'static {
    fn perform_action(&self, action: WindowAction);
}
