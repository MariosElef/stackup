use std::{io, time::Duration};

use anyhow::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event},
};

use super::{
    components::{autofix_modal, choice_modal, help_dialog, palette},
    composables::use_terminal::with_terminal,
    features::{
        create::{self, github_modal, requirements_modal, selection::Selection},
        home,
        projects::{self, tier_modal},
    },
    layouts,
    stores::app::{App, Outcome},
    types::{Mode, Page},
};
use crate::services::projects::registry;

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while app.outcome == Outcome::Running {
        terminal.draw(|f| draw(f, app))?;
        // Waits for a key, but not for long: the logo moves and a running action has output. A
        // program running here (lazygit) answers every key, so it is looked at much more often:
        // what it printed in reply shows at once, not on the next slow beat.
        let wait = if app.run.as_ref().is_some_and(|r| r.live()) {
            8
        } else if app.farewell.is_some() {
            // The goodbye is an animation: about 30 frames a second.
            30
        } else {
            100
        };
        if event::poll(Duration::from_millis(wait))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }
        app.pump();
    }
    Ok(())
}

/// `stackup` with no command: the home menu, then the projects. A new project is made inside
/// it, on the in-app terminal, so there is nothing left to do when it ends.
pub fn menu() -> Result<()> {
    let mut app = App::new(registry::home(), App::installed_tools());
    with_terminal(|terminal| event_loop(terminal, &mut app))?;
    Ok(())
}

/// `stackup new NAME` with choices missing: straight to the wizard. `None` means the user left
/// without creating anything.
pub fn run(name: &str, initial: Selection) -> Result<Option<Selection>> {
    let mut app = App::new(registry::home(), App::installed_tools());
    app.wizard_only = true;
    app.selection = initial;
    app.open_wizard(name);
    with_terminal(|terminal| event_loop(terminal, &mut app))?;
    Ok(matches!(app.outcome, Outcome::Start { .. }).then_some(app.selection))
}

pub fn draw(f: &mut Frame, app: &App) {
    let footer = app.footer();
    layouts::default::render(f, &footer, |f, area| match app.page {
        _ if app.farewell.is_some() => {
            if let Some(farewell) = &app.farewell {
                projects::farewell::render(f, area, farewell);
            }
        }
        Page::Home => home::page::render(f, area, app),
        Page::Projects => projects::list_page::render(f, area, app),
        Page::Project | Page::Run => projects::project_page::render(f, area, app),
        Page::Install => create::install_page::render(f, area, app),
        Page::New => {
            if let Some(wizard) = &app.wizard {
                create::page::render(f, area, wizard);
            }
        }
    });
    if app.help {
        help_dialog::render_app(f, f.area());
    }
    if let Some(modal) = &app.tier_modal {
        tier_modal::render(f, f.area(), modal);
    }
    if let Some(p) = &app.palette {
        palette::render(f, f.area(), p);
    }
    // A version question sits alone: the question it belongs to is not drawn around it.
    if let Some(modal) = &app.choice {
        choice_modal::render(f, f.area(), modal);
    } else if let Some(modal) = &app.install {
        requirements_modal::render(f, f.area(), modal);
    } else if let Some(modal) = &app.autofix {
        autofix_modal::render(f, f.area(), modal);
    } else if let Some(modal) = &app.github {
        github_modal::render(f, f.area(), modal);
    }
    if let Some(wizard) = &app.wizard
        && wizard.mode == Mode::Help
    {
        help_dialog::render_wizard(f, f.area());
    }
}

/// The wizard alone, for the tests that look at what it shows.
#[cfg(test)]
pub fn draw_wizard(f: &mut Frame, wizard: &super::features::create::wizard::Wizard) {
    layouts::default::render(f, &wizard.footer(), |f, area| {
        create::page::render(f, area, wizard);
    });
    if wizard.mode == Mode::Help {
        help_dialog::render_wizard(f, f.area());
    }
}
