mod edit;
mod init;
mod login;
mod main;
mod utils;

pub use edit::render_edit_popup;
pub use init::render_init;
pub use login::render_login;
pub use main::render_main;

use crate::app::App;
use crate::app::CurrentScreen;
use ratatui::Frame;

// Entry point
pub fn ui(frame: &mut Frame, app: &App) {
    match app.current_screen {
        CurrentScreen::Login => render_login(frame, app),
        CurrentScreen::Init => render_init(frame, app),
        _ => render_main(frame, app),
    }

    if app.currently_editing.is_some() {
        // Please review this. Why is this different from above?
        render_edit_popup(frame, app);
    }
}
