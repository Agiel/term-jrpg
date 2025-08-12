use std::u32;

use hecs::With;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Gauge, List, Paragraph, Row, Table, Wrap},
};

use crate::app::{
    App, Burning, CurrentScreen, GameState, Health, Hostile, Job, Level, Name, Party, Stats,
    StyledLine, StyledSpan, get_log,
};

pub fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_title(frame, chunks[0]);
    draw_field(frame, chunks[1], app);
    draw_main(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);
    draw_popup(frame, app);
}

fn draw_title(frame: &mut Frame, rect: Rect) {
    let title_block = Block::default()
        .title("Terminal JRPG")
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled(
        "The net is vast and full of terrors",
        Style::default().fg(Color::Green),
    ))
    .block(title_block);

    frame.render_widget(title, rect);
}

fn draw_field(frame: &mut Frame, rect: Rect, app: &App) {
    match app.game_state {
        GameState::Combat => {
            let combat_chunks = Layout::horizontal(vec![
                Constraint::Fill(1),
                Constraint::Fill(2),
                Constraint::Length(20),
            ])
            .split(rect);
            draw_log(frame, combat_chunks[0], app);
            draw_enemies(frame, combat_chunks[1], app);
            draw_order(frame, combat_chunks[2], app);
        }
        _ => unimplemented!(),
    }
}

fn draw_log(frame: &mut Frame, rect: Rect, _app: &App) {
    let log = get_log();
    let lines = log
        .iter()
        .map(|StyledLine(spans, alignment)| {
            Line::default()
                .spans(
                    spans
                        .iter()
                        .map(|StyledSpan(text, style)| Span::styled(text, *style)),
                )
                .alignment(*alignment)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().title("Log").borders(Borders::ALL))
            .scroll((
                ((log.len() as u16).saturating_sub(rect.height.saturating_sub(2))),
                0,
            )),
        rect,
    );
}

struct EnemyInfo {
    name: &'static str,
    level: u8,
    health: u32,
    max_health: u32,
    status: String,
    target: bool,
}

fn draw_enemies(frame: &mut Frame, rect: Rect, app: &App) {
    let enemy_info = app
        .world
        .query::<With<(&Name, &Level, &Health, &Stats), &Hostile>>()
        .iter()
        .map(
            |(entity, (&Name(name), &Level(level), &Health(health), stats))| {
                let mut status = String::new();
                if let Ok(burning) = app.world.get::<&Burning>(entity) {
                    status += &format!("🔥{}", burning.0);
                }

                let target = if matches!(app.current_screen, CurrentScreen::Target) {
                    match app.selected_target {
                        None => app.targets.contains(&entity),
                        Some(selected) => app.targets[selected] == entity,
                    }
                } else {
                    false
                };

                EnemyInfo {
                    name,
                    level,
                    health,
                    max_health: stats.max_health,
                    status,
                    target,
                }
            },
        )
        .collect::<Vec<_>>();

    let enemy_chunks = Layout::horizontal(vec![Constraint::Length(20); enemy_info.len()])
        .flex(Flex::Center)
        .split(rect);

    enemy_info.iter().enumerate().for_each(|(i, info)| {
        let centered = Layout::vertical(vec![Constraint::Length(1), Constraint::Length(4)])
            .flex(Flex::Center)
            .split(enemy_chunks[i]);

        if info.target {
            frame.render_widget(Text::raw("⮟").centered(), centered[0]);
        }

        frame.render_widget(
            Block::default()
                .title(format!("{} Lv.{}", info.name, info.level))
                .borders(Borders::ALL),
            centered[1],
        );
        let info_chunks = Layout::vertical(vec![Constraint::Length(1), Constraint::Fill(1)])
            .margin(1)
            .split(centered[1]);
        let mut chunk = 0;
        frame.render_widget(
            Gauge::default()
                .ratio(info.health as f64 / info.max_health as f64)
                .label(format!("{}/{}", info.health, info.max_health))
                .gauge_style(Color::Red),
            info_chunks[chunk],
        );

        chunk += 1;
        frame.render_widget(
            Paragraph::new(Text::raw(info.status.as_str())),
            info_chunks[chunk],
        );
    });
}

fn draw_order(frame: &mut Frame, rect: Rect, app: &App) {
    let Some(next_up) = app.next_up.clone() else {
        return;
    };
    frame.render_widget(
        Paragraph::new(Text::from(
            next_up
                .take(rect.height as usize - 2)
                .enumerate()
                .map(|(n, i)| {
                    let name = app.world.get::<&Name>(i.entity).unwrap().0;
                    let mut line = if i.hostile {
                        Line::raw(name).right_aligned().style(Color::LightRed)
                    } else {
                        Line::raw(name).left_aligned().style(Color::Green)
                    };
                    if n == 0 {
                        line = line.bold()
                    }
                    line
                })
                .collect::<Vec<_>>(),
        ))
        .block(Block::default().title("Next up").borders(Borders::ALL)),
        rect,
    );
}

fn draw_main(frame: &mut Frame, rect: Rect, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Fill(1)])
        .split(rect);

    draw_actions(frame, main_chunks[0], app);
    draw_party(frame, main_chunks[1], app);

    match app.current_screen {
        CurrentScreen::Skill => draw_skills(frame, rect, app),
        CurrentScreen::Item => draw_items(frame, rect, app),
        _ => (),
    }
}

fn draw_actions(frame: &mut Frame, rect: Rect, app: &mut App) {
    let action_block = Block::default()
        .title("Actions ↓↑")
        .borders(Borders::ALL)
        .style(Style::default());

    let items = app
        .action_list_items
        .iter()
        .map(|i| i.text)
        .collect::<Vec<_>>();
    let action_list = List::default()
        .items(items)
        .highlight_style(Style::new().reversed())
        .block(action_block);

    frame.render_stateful_widget(action_list, rect, &mut app.action_list_state);
}

fn draw_party(frame: &mut Frame, rect: Rect, app: &App) {
    let party_block = Block::default()
        .title("Party")
        .borders(Borders::ALL)
        .style(Style::default());

    frame.render_widget(party_block, rect);

    let party_chunks = Layout::vertical([Constraint::Length(1); 3])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(rect);

    app.world
        .query::<With<(&Name, &Health, &Stats, &Job), &Party>>()
        .iter()
        .enumerate()
        .for_each(
            |(i, (entity, (&Name(name), &Health(health), stats, job)))| {
                let character_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(16),
                        Constraint::Length(16),
                        Constraint::Fill(1),
                    ])
                    .spacing(2)
                    .split(party_chunks[i]);

                let mut chunk = 0;
                if matches!(app.current_screen, CurrentScreen::Target)
                    && let Some(selected) = app.selected_target
                    && app.targets[selected] == entity
                {
                    frame.render_widget(Paragraph::new("⮞"), character_chunks[chunk]);
                }

                chunk += 1;
                let mut name =
                    Paragraph::new(Text::styled(name, Color::Gray)).block(Block::default());
                if let Some(ent) = app.turn
                    && ent == entity
                {
                    name = name.bold();
                }
                frame.render_widget(name, character_chunks[chunk]);

                chunk += 1;
                frame.render_widget(
                    Gauge::default()
                        .ratio(health as f64 / stats.max_health as f64)
                        .label(format!("{}/{}", health, stats.max_health))
                        .gauge_style(Color::Red),
                    character_chunks[chunk],
                );

                chunk += 1;
                frame.render_widget(
                    Paragraph::new(match job {
                        Job::Gunslinger { ammo } => {
                            Line::styled(format!("⁍ {}", ammo), Color::DarkGray)
                        }
                        Job::Netrunner { ram, heat } => Line::from(vec![
                            Span::styled(format!("{}GB", ram), Color::Blue),
                            Span::styled(format!("  {}ºC", heat), Color::LightRed),
                        ]),
                        Job::Technopriest { prayers } => {
                            Line::styled(format!("✠ {}", prayers), Color::LightMagenta)
                        }
                        Job::Clairvoyant { sun, moon } => Line::from(vec![
                            Span::styled(format!("☀ {}", sun), Color::Yellow),
                            Span::styled(format!("  ☽︎ {}", moon), Color::Magenta),
                        ]),
                        Job::Nanovampire { battery } => {
                            // TODO: Find less risky character? This one probably won't always fill two cells.
                            Line::styled(format!("⚡{}%", battery), Color::LightYellow)
                        }
                        Job::None => Line::raw(""),
                    }),
                    character_chunks[chunk],
                )
            },
        );
}

fn draw_skills(frame: &mut Frame, rect: Rect, app: &App) {
    let rect = Layout::horizontal(vec![Constraint::Length(20)])
        .horizontal_margin(4)
        .split(
            Layout::vertical(vec![Constraint::Length(6)])
                .flex(Flex::End)
                .vertical_margin(frame.area().height - rect.top() - 1)
                .split(frame.area())[0],
        )[0];
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Block::default().title("Skills ↓↑").borders(Borders::ALL),
        rect,
    );
}

fn draw_items(frame: &mut Frame, rect: Rect, app: &mut App) {
    let rect = Layout::horizontal(vec![Constraint::Length(20)])
        .horizontal_margin(4)
        .split(
            Layout::vertical(vec![Constraint::Length(6)])
                .flex(Flex::End)
                .vertical_margin(frame.area().height - rect.top() - 1)
                .split(frame.area())[0],
        )[0];
    frame.render_widget(Clear, rect);

    let widths = vec![Constraint::Fill(1), Constraint::Length(4)];
    let rows = app
        .consumables
        .iter()
        .map(|i| {
            Row::new(vec![
                Cell::from(i.name),
                Cell::from(Line::from(i.amount.to_string()).right_aligned()),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        Table::new(rows, widths)
            .row_highlight_style(Style::default().reversed())
            .block(Block::default().title("Items ↓↑").borders(Borders::ALL)),
        rect,
        &mut app.consumable_list_state,
    );
}

fn draw_footer(frame: &mut Frame, rect: Rect, app: &App) {
    let current_navigation_text = vec![
        // The first half of the text
        match app.current_screen {
            CurrentScreen::Main => Span::styled("Select Action", Style::default().fg(Color::Green)),
            CurrentScreen::Target => {
                Span::styled("Select Target", Style::default().fg(Color::Green))
            }
            CurrentScreen::Skill => Span::styled("Select Skill", Style::default().fg(Color::Green)),
            CurrentScreen::Item => Span::styled("Select Item", Style::default().fg(Color::Green)),
            CurrentScreen::Exiting => Span::styled("Exiting", Style::default().fg(Color::LightRed)),
        }
        .to_owned(),
    ];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_keys_hint = {
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                "(q) to quit / (↓↑) to select action",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Skill => Span::styled(
                "(esc) to cancel / (↓↑) to select skill",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Item => Span::styled(
                "(esc) to cancel / (↓↑) to select item",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Target => Span::styled(
                "(esc) to cancel / (←→) to select target",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Exiting => Span::styled("(q) to quit", Style::default().fg(Color::Red)),
        }
    };

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rect);

    frame.render_widget(mode_footer, footer_chunks[0]);
    frame.render_widget(key_notes_footer, footer_chunks[1]);
}

fn draw_popup(frame: &mut Frame, app: &App) {
    if let CurrentScreen::Exiting = app.current_screen {
        let popup_block = Block::default().title("Really quit?").borders(Borders::ALL);

        let exit_text = Text::styled(
            "Press (q) again to confirm",
            Style::default().fg(Color::Red),
        );
        // the `trim: false` will stop the text from being cut off when over the edge of the block
        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false });

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(exit_paragraph, area);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}
