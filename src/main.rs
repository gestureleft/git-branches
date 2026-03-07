use std::io::stdout;

use crossterm::{
    cursor::MoveToColumn,
    event::{Event::Key, KeyCode, KeyEvent},
    execute,
    terminal::enable_raw_mode,
};
use git2::Repository;
use ratatui::{
    DefaultTerminal, Frame, Terminal, TerminalOptions,
    prelude::CrosstermBackend,
    style::{Color, Stylize},
    text::Span,
    widgets::{List, ListItem},
};

const TEXT_SELECTED_FG_COLOUR: Color = Color::Black;
const TEXT_SELECTED_BG_COLOUR: Color = Color::White;

const TEXT_UNSELECTED_FG_COLOUR: Color = Color::White;
const TEXT_UNSELECTED_BG_COLOUR: Color = Color::Reset;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let repo = Repository::open(".")?;

    let branch_names: Vec<String> = repo
        .branches(None)?
        .take(8)
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .map(|(branch, _)| {
            branch
                .name()
                .unwrap()
                .unwrap_or("[unnamed_branch]")
                .to_owned()
        })
        .collect();

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: ratatui::Viewport::Inline(8),
        },
    )?;

    let app = App {
        branch_names,
        selected_branch_index: 0,
    };
    let app_result = app.run(terminal);

    crossterm::terminal::disable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, MoveToColumn(0))?;
    println!();

    let app = app_result?;
    let selected_branch_name = &app.branch_names[app.selected_branch_index];
    println!("Selected: {}", selected_branch_name);
    Ok(())
}

struct App {
    branch_names: Vec<String>,
    selected_branch_index: usize,
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<Self> {
        loop {
            terminal.draw(|f| render(f, &self))?;
            let Key(KeyEvent { code, .. }) = crossterm::event::read()? else {
                continue;
            };
            if code == KeyCode::Enter {
                break Ok(self);
            }
            if code == KeyCode::Down {
                self.selected_branch_index = self.selected_branch_index + 1;
                if self.selected_branch_index >= self.branch_names.len() {
                    self.selected_branch_index = 0;
                }
            }
            if code == KeyCode::Up {
                if self.selected_branch_index == 0 {
                    self.selected_branch_index = self.branch_names.len() - 1;
                } else {
                    self.selected_branch_index = self.selected_branch_index - 1;
                }
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let list = List::new(
        app.branch_names
            .iter()
            .enumerate()
            .map(|(branch_index, branch_name)| {
                ListItem::new(
                    Span::raw(format!(" {}  {}", branch_index, branch_name))
                        .fg(if branch_index == app.selected_branch_index {
                            TEXT_SELECTED_FG_COLOUR
                        } else {
                            TEXT_UNSELECTED_FG_COLOUR
                        })
                        .bg(if branch_index == app.selected_branch_index {
                            TEXT_SELECTED_BG_COLOUR
                        } else {
                            TEXT_UNSELECTED_BG_COLOUR
                        }),
                )
            }),
    );
    frame.render_widget(list, frame.area());
}
