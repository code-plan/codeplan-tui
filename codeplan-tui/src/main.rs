use std::collections::hash_map;
use std::fs;
use std::future::Future;
use std::io;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, ser, Serialize};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    Terminal,
    text::{Span, Spans},
    widgets::{
        Block, Borders, BorderType, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
};

const TASK_PATH: &str = "./cache/task.json";
const COMMENT_PATH: &str = "./cache/comment.json";
const PROJECT_PATH: &str = "./cache/project.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    id: usize,
    project: String,
    content_preview: String,
    content: String,
    begin_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    finish_date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Comment {
    id: usize,
    task_preview: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Project {
    id: usize,
    name: String,
    customer_name: String,
    customer_document: String,
    customer_contact: String,
    created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Monitor,
    Comments,
    Projects,
    License,
    Error,
}

impl<'a> From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Monitor => 1,
            MenuItem::Comments => 2,
            MenuItem::Projects => 3,
            MenuItem::License => 4,
            MenuItem::Error => 5,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Início", "Tasks", "Comentários (Tasks)", "Projetos", "Licença", "Sair"];
    let mut active_menu_item = MenuItem::Home;
    let mut tasks_list_state = ListState::default();
    tasks_list_state.select(Some(0));
    let mut comments_list_state = ListState::default();
    comments_list_state.select(Some(0));
    let mut projects_list_state = ListState::default();
    projects_list_state.select(Some(0));


    let home_position: usize = 0;
    let monitor_position: usize = 1;
    let comments_position: usize = 2;
    let projects_position: usize = 3;
    let license_position: usize = 4;

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                        .as_ref(),
                )
                .split(size);

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::DarkGray)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL).border_type(BorderType::Rounded))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::White))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                MenuItem::Home => {
                    rect.render_widget(render_home(), chunks[1]);
                    rect.render_widget(render_options("Nenhuma ação disponível"), chunks[2]);
                }
                MenuItem::Monitor => {
                    let tasks_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(15), Constraint::Percentage(85)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_monitor(&tasks_list_state);
                    rect.render_stateful_widget(left, tasks_chunks[0], &mut tasks_list_state);
                    rect.render_widget(right, tasks_chunks[1]);
                    rect.render_widget(render_options("(f) Marcar como concluída | (d) Deletar"), chunks[2]);
                }
                MenuItem::Comments => {
                    let tasks_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(15), Constraint::Percentage(85)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_comments(&comments_list_state);
                    rect.render_stateful_widget(left, tasks_chunks[0], &mut comments_list_state);
                    rect.render_widget(right, tasks_chunks[1]);
                    rect.render_widget(render_options("Nenhuma ação disponível"), chunks[2]);
                }
                MenuItem::Projects => {
                    let projects_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(15), Constraint::Percentage(85)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_projects(&projects_list_state);
                    rect.render_stateful_widget(left, projects_chunks[0], &mut projects_list_state);
                    rect.render_widget(right, projects_chunks[1]);
                    rect.render_widget(render_options("Nenhuma ação disponível"), chunks[2]);
                }
                MenuItem::License => {
                    rect.render_widget(render_license(), chunks[1]);
                    rect.render_widget(render_options("Nenhuma ação disponível"), chunks[2]);
                }
                MenuItem::Error => {
                    rect.render_widget(render_error("Ocorreu um erro :(", "Para mais informações, entre em contato", "com o desenvolvedor da aplicação."), chunks[1]);
                    rect.render_widget(render_options("Nenhuma ação disponível"), chunks[2]);
                }
            }
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('s') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('i') => active_menu_item = MenuItem::Home,
                KeyCode::Char('t') => active_menu_item = MenuItem::Monitor,
                KeyCode::Char('c') => active_menu_item = MenuItem::Comments,
                KeyCode::Char('p') => active_menu_item = MenuItem::Projects,
                KeyCode::Char('l') => active_menu_item = MenuItem::License,
                KeyCode::Char('u') => {
                    Command::new("./codeplan-updater").stderr(Stdio::null()).spawn().expect("ls command failed to start");
                }
                KeyCode::Char('f') => {
                    let delete_active_position: usize = From::<MenuItem>::from(active_menu_item);
                    if delete_active_position == monitor_position {
                        complete_task(&mut tasks_list_state);
                    }
                }
                KeyCode::Char('d') => {
                    let delete_active_position: usize = From::<MenuItem>::from(active_menu_item);
                    if delete_active_position == monitor_position {
                        delete_task(&mut tasks_list_state);
                    }
                }
                KeyCode::Down => {
                    let down_active_position: usize = From::<MenuItem>::from(active_menu_item);
                    if down_active_position == monitor_position {
                        if let Some(selected) = tasks_list_state.selected() {
                            let amount_tasks = read_db_task().expect("can fetch task list").len();
                            if selected >= amount_tasks - 1 {
                                tasks_list_state.select(Some(0));
                            } else {
                                tasks_list_state.select(Some(selected + 1));
                            }
                        }
                    } else if down_active_position == comments_position {
                        if let Some(selected) = comments_list_state.selected() {
                            let amount_comments = read_db_comment().expect("can fetch task list").len();
                            if selected >= amount_comments - 1 {
                                comments_list_state.select(Some(0));
                            } else {
                                comments_list_state.select(Some(selected + 1));
                            }
                        }
                    } else if down_active_position == projects_position {
                        if let Some(selected) = projects_list_state.selected() {
                            let amount_comments = read_db_project().expect("can fetch task list").len();
                            if selected >= amount_comments - 1 {
                                projects_list_state.select(Some(0));
                            } else {
                                projects_list_state.select(Some(selected + 1));
                            }
                        }
                    }
                }
                KeyCode::Up => {
                    let up_active_position: usize = From::<MenuItem>::from(active_menu_item);
                    if up_active_position == monitor_position {
                        if let Some(selected) = tasks_list_state.selected() {
                            let amount_tasks = read_db_task().expect("can fetch task list").len();
                            if selected > 0 {
                                tasks_list_state.select(Some(selected - 1));
                            } else {
                                tasks_list_state.select(Some(amount_tasks - 1));
                            }
                        }
                    } else if up_active_position == comments_position {
                        if let Some(selected) = comments_list_state.selected() {
                            let amount_comments = read_db_comment().expect("can fetch task list").len();
                            if selected > 0 {
                                comments_list_state.select(Some(selected - 1));
                            } else {
                                comments_list_state.select(Some(amount_comments - 1));
                            }
                        }
                    } else if up_active_position == projects_position {
                        if let Some(selected) = projects_list_state.selected() {
                            let amount_comments = read_db_project().expect("can fetch task list").len();
                            if selected > 0 {
                                projects_list_state.select(Some(selected - 1));
                            } else {
                                projects_list_state.select(Some(amount_comments - 1));
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_error<'a>(title: &'a str, msg: &'a str, msg2: &'a str) -> Paragraph<'a> {
    let error = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            title,
            Style::default().fg(Color::Red).add_modifier(Modifier::RAPID_BLINK),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            msg,
            Style::default().fg(Color::Red),
        )]),
        Spans::from(vec![Span::styled(
            msg2,
            Style::default().fg(Color::Red),
        )]),
    ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Erro")
                .border_type(BorderType::Rounded),
        );
    error
}

fn render_options<'a>(text: &'a str) -> Paragraph<'a> {
    let options = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Opções")
                .border_type(BorderType::Rounded),
        );
    options
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::styled(
            "Codeplan Terminal UI",
            Style::default().fg(Color::White).add_modifier(Modifier::RAPID_BLINK),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Pressione 'i' para acessar a página inicial, 't' para acessar seu monitor de tasks,")]),
        Spans::from(vec![Span::raw("'s' para sair do programa e 'u' para sincronizar os dados com o servidor.")]),
    ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Início")
                .border_type(BorderType::Rounded),
        );
    home
}

fn render_license<'a>() -> Paragraph<'a> {
    let license = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "Codeplan TUI by Open Build 2021 - todos os direitos reservados.",
            Style::default().fg(Color::White).add_modifier(Modifier::RAPID_BLINK),
        )]),
    ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Licença")
                .border_type(BorderType::Rounded),
        );
    license
}

fn render_monitor<'a>(tasks_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let tasks = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Monitor")
        .border_type(BorderType::Rounded);

    let tasks_list = read_db_task().expect("can fetch task list");
    let items: Vec<_> = tasks_list
        .iter()
        .map(|task| {
            ListItem::new(Spans::from(vec![Span::styled(
                task.content_preview.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_task = tasks_list
        .get(
            tasks_list_state
                .selected()
                .expect("there is always a selected task"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(tasks).highlight_style(
        Style::default()
            .bg(Color::White)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    let task_detail = Table::new(vec![
        Row::new(vec![
            Cell::from(Span::raw(selected_task.project)),
            Cell::from(Span::raw(selected_task.content.to_string())),
            Cell::from(Span::raw(selected_task.begin_date.to_string())),
            Cell::from(Span::raw(selected_task.end_date.to_string())),
        ])
    ])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "Projeto",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Descrição",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Início",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Entrega",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Detalhes")
                .border_type(BorderType::Rounded),
        )
        .widths(&[
            Constraint::Percentage(10),
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]);

    (list, task_detail)
}

fn render_comments<'a>(comments_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let comments = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Comentários")
        .border_type(BorderType::Rounded);

    let comments_list = read_db_comment().expect("can fetch comments list");
    let items: Vec<_> = comments_list
        .iter()
        .map(|comment| {
            ListItem::new(Spans::from(vec![Span::styled(
                comment.task_preview.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_comment = comments_list
        .get(
            comments_list_state
                .selected()
                .expect("there is always a selected comment"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(comments).highlight_style(
        Style::default()
            .bg(Color::White)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    let comment_detail = Table::new(vec![
        Row::new(vec![
            Cell::from(Span::raw(selected_comment.content.to_string())),
            Cell::from(Span::raw(selected_comment.created_at.to_string())),
        ])
    ])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "Comentário",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Comentado em",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Detalhes")
                .border_type(BorderType::Rounded),
        )
        .widths(&[
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ]);

    (list, comment_detail)
}

fn render_projects<'a>(projects_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let projects = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Projetos")
        .border_type(BorderType::Rounded);

    let projects_list = read_db_project().expect("can fetch projects list");
    let items: Vec<_> = projects_list
        .iter()
        .map(|project| {
            ListItem::new(Spans::from(vec![Span::styled(
                project.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_project = projects_list
        .get(
            projects_list_state
                .selected()
                .expect("there is always a selected comment"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(projects).highlight_style(
        Style::default()
            .bg(Color::White)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );
    let project_detail = Table::new(vec![
        Row::new(vec![
            Cell::from(Span::raw(selected_project.customer_name.to_string())),
            Cell::from(Span::raw(selected_project.customer_document.to_string())),
            Cell::from(Span::raw(selected_project.customer_contact.to_string())),
            Cell::from(Span::raw(selected_project.created_at.to_string())),
        ])
    ])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "Cliente",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Doc. Cliente",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Con. Cliente",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Criado",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Detalhes")
                .border_type(BorderType::Rounded),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ]);

    (list, project_detail)
}

fn read_db_task() -> Result<Vec<Task>, Error> {
    let db_content = fs::read_to_string(TASK_PATH)?;
    let parsed: Vec<Task> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_db_comment() -> Result<Vec<Comment>, Error> {
    let db_content = fs::read_to_string(COMMENT_PATH)?;
    let parsed: Vec<Comment> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_db_project() -> Result<Vec<Project>, Error> {
    let db_content = fs::read_to_string(PROJECT_PATH)?;
    let parsed: Vec<Project> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn complete_task(tasks_list_state: &mut ListState) -> Result<(), Error> {
    if let Some(mut selected) = tasks_list_state.selected() {
        selected = selected + 1;
        Command::new("./codeplan-task-control").args(&["-complete", &selected.to_string()]).stderr(Stdio::null()).spawn().expect("codeplan-task-control command failed to start");
    }
    Ok(())
}

fn delete_task(tasks_list_state: &mut ListState) -> Result<(), Error> {
    if let Some(mut selected) = tasks_list_state.selected() {
        selected = selected + 1;
        Command::new("./codeplan-task-control").args(&["-delete", &selected.to_string()]).stderr(Stdio::null()).spawn().expect("codeplan-task-control command failed to start");
    }
    Ok(())
}
