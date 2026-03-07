use std::io::stdout;

use crossterm::{
    cursor::MoveToColumn,
    event::{Event::Key, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::enable_raw_mode,
};
use git2::{BranchType, Repository};
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

    let branch_names: Vec<String> = branches_sorted_by_commit_date(&repo)?;

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

    let app_outcome = app_result?;
    if let Some(selected_branch_name) = app_outcome {
        checkout_branch_strict(&repo, &selected_branch_name)?;
    }
    Ok(())
}

struct App {
    branch_names: Vec<String>,
    selected_branch_index: usize,
}

type AppOutcome = Option<String>;

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<AppOutcome> {
        loop {
            terminal.draw(|f| render(f, &self))?;
            let Key(KeyEvent {
                code, modifiers, ..
            }) = crossterm::event::read()?
            else {
                continue;
            };
            if code == KeyCode::Enter {
                break Ok(self
                    .branch_names
                    .into_iter()
                    .nth(self.selected_branch_index));
            }
            if (code == KeyCode::Char('c') || code == KeyCode::Char('d'))
                && modifiers.contains(KeyModifiers::CONTROL)
            {
                break Ok(None);
            }
            if let KeyCode::Char(char) = code
                && let Some(digit) = char.to_digit(10).map(|d| d as usize)
                && let Some(selected_branch_hame) = self.branch_names.clone().into_iter().nth(digit)
            {
                break Ok(Some(selected_branch_hame));
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

fn branches_sorted_by_commit_date(repo: &Repository) -> Result<Vec<String>, git2::Error> {
    let mut branches: Vec<(String, i64)> = repo
        .branches(Some(BranchType::Local))?
        .filter_map(|b| {
            let (branch, _) = b.ok()?;
            let name = branch.name().ok()??.to_string();
            let commit = branch.get().peel_to_commit().ok()?;
            let time = commit.time().seconds();
            Some((name, time))
        })
        .collect();

    branches.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(branches.into_iter().map(|(name, _)| name).take(8).collect())
}

fn checkout_branch_strict(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let ref_name = branch.get().name().unwrap();

    let object = repo.revparse_single(&format!("refs/heads/{}", branch_name))?;
    repo.checkout_tree(&object, None)?;
    repo.set_head(ref_name)?;

    Ok(())
}
