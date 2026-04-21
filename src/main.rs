use std::{io::stdout, iter};

use crossterm::{
    cursor::MoveToColumn,
    event::{Event::Key, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::enable_raw_mode,
};
use git2::{BranchType, Object, Repository};
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

    let branches = branches_sorted_by_commit_date(&repo)?;

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: ratatui::Viewport::Inline(8),
        },
    )?;

    let app = App {
        branches,
        selected_branch_index: 0,
        search_query: "".into(),
    };
    let app_result = app.run(terminal);

    crossterm::terminal::disable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, MoveToColumn(0))?;
    println!();

    let app_outcome = app_result?;
    if let Some(selected_branch) = app_outcome {
        repo.checkout_tree(&selected_branch.object, None)?;
        repo.set_head(&format!("refs/heads/{}", selected_branch.name))?;
    }
    Ok(())
}

#[derive(Clone)]
struct Branch<'repo> {
    name: String,
    object: Object<'repo>,
}

struct App<'repo> {
    branches: Vec<Branch<'repo>>,
    selected_branch_index: usize,
    search_query: String,
}

type AppOutcome<'repo> = Option<Branch<'repo>>;

impl<'repo> App<'repo> {
    fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<AppOutcome<'repo>> {
        loop {
            terminal.draw(|f| render(f, &self))?;
            let Key(KeyEvent {
                code, modifiers, ..
            }) = crossterm::event::read()?
            else {
                continue;
            };
            // Select current branch
            if code == KeyCode::Enter {
                break Ok(self
                    .filtered_branches()
                    .nth(self.selected_branch_index)
                    .cloned());
            }
            let ctrl = modifiers.contains(KeyModifiers::CONTROL);
            // Exit with ctrl + c / d
            if (code == KeyCode::Char('c') || code == KeyCode::Char('d')) && ctrl {
                break Ok(None);
            }

            let filtered_branches_count = self.filtered_branches().count();
            let option = modifiers.contains(KeyModifiers::ALT);
            // Code is option + a number -> selected a branch
            if let KeyCode::Char(char) = code
                && option
                && let Some(digit) = char.to_digit(10).map(|d| d as usize)
                && let Some(selected_branch_hame) = self.filtered_branches().nth(digit).cloned()
            {
                break Ok(Some(selected_branch_hame));
            }
            let emacs_down = ctrl && matches!(code, KeyCode::Char('n'));
            // Navigate down with arrow or emacs binding
            if code == KeyCode::Down || emacs_down {
                self.selected_branch_index = self.selected_branch_index + 1;
                if self.selected_branch_index >= filtered_branches_count {
                    self.selected_branch_index = 0;
                }
                continue;
            }
            // Navigate up with arrow or emacs binding
            let emacs_up = ctrl && matches!(code, KeyCode::Char('p'));
            if code == KeyCode::Up || emacs_up {
                if self.selected_branch_index == 0 {
                    self.selected_branch_index = filtered_branches_count - 1;
                } else {
                    self.selected_branch_index = self.selected_branch_index - 1;
                }
                continue;
            }

            // Append to search term
            if let KeyCode::Char(char) = code {
                self.search_query.push(char);
                continue;
            }

            // Delete last character appended to search term
            if let KeyCode::Backspace = code {
                self.search_query.pop();
                continue;
            }
        }
    }

    fn filtered_branches(self: &Self) -> impl Iterator<Item = &Branch<'repo>> {
        self.branches
            .iter()
            .filter(|branch| branch.name.starts_with(&self.search_query))
    }
}

fn render(frame: &mut Frame, app: &App) {
    let list =
        List::new(
            iter::once(ListItem::new(Span::raw(format!(
                "Search term: {}",
                &app.search_query
            ))))
            .chain(app.filtered_branches().enumerate().map(
                |(branch_index, branch)| {
                    ListItem::new(
                        Span::raw(format!(" {}  {}", branch_index, branch.name))
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
                },
            )),
        );
    frame.render_widget(list, frame.area());
}

fn branches_sorted_by_commit_date<'repo>(
    repo: &'repo Repository,
) -> Result<Vec<Branch<'repo>>, git2::Error> {
    let mut branches: Vec<(String, Object, i64)> = repo
        .branches(Some(BranchType::Local))?
        .filter_map(|b| {
            let (branch, _) = b.ok()?;
            let binding = branch.get().peel_to_commit().ok()?;
            let object = binding.as_object();
            let name = branch.name().ok()??.to_string();
            let commit = branch.get().peel_to_commit().ok()?;
            let time = commit.time().seconds();
            Some((name, object.clone(), time))
        })
        .collect();

    branches.sort_by(|a, b| b.2.cmp(&a.2));

    Ok(branches
        .into_iter()
        .map(|(name, object, _)| Branch { name, object })
        .collect())
}
