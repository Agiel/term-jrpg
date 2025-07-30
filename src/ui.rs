use hecs::With;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, Paragraph, Wrap},
};

use crate::app::{App, CurrentScreen, Health, Job, Name, Party, Stats};

pub fn ui(frame: &mut Frame, app: &App) {
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

fn draw_field(frame: &mut Frame, rect: Rect, app: &App) {}

fn draw_main(frame: &mut Frame, rect: Rect, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Fill(1)])
        .split(rect);

    let action_block = Block::default()
        .title("Actions ↓↑")
        .borders(Borders::ALL)
        .style(Style::default());

    let actions = ["Skill", "Melee", "Item"]; // , "Swap"];
    let action_list = List::default().items(actions).block(action_block);

    frame.render_widget(action_list, main_chunks[0]);

    let party_block = Block::default()
        .title("Party")
        .borders(Borders::ALL)
        .style(Style::default());

    frame.render_widget(party_block, main_chunks[1]);

    let party_chunks = Layout::vertical([Constraint::Length(1); 3])
        .vertical_margin(1)
        .horizontal_margin(2)
        .split(main_chunks[1]);

    app.world
        .query::<With<(&Name, &Health, &Stats, &Job), &Party>>()
        .iter()
        .enumerate()
        .for_each(|(i, (entity, (Name(name), Health(health), stats, job)))| {
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
            if let Some(ent) = app.turn
                && ent == entity
            {
                frame.render_widget(Paragraph::new("⮞"), character_chunks[chunk]);
            }

            chunk += 1;
            frame.render_widget(
                Paragraph::new(Text::styled(name.as_str(), Color::Gray)).block(Block::default()),
                character_chunks[chunk],
            );

            chunk += 1;
            frame.render_widget(
                Gauge::default()
                    .ratio(*health as f64 / stats.max_health as f64)
                    .label(format!("{}/{}", *health, stats.max_health))
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
                    Job::Oracle { sun, moon } => Line::from(vec![
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
        });
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
