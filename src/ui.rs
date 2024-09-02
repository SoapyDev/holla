use crate::app::{App, CurrentScreen};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::prelude::{Direction, Line, Style, Text};
use ratatui::style::Color;
use ratatui::text::Span;
use ratatui::widgets::{
    Block, Borders, List, Padding, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
};
use ratatui::Frame;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled("Holla", Style::default().fg(Color::Gray)))
        .block(title_block)
        .centered();

    frame.render_widget(title, chunk[0]);

    let mut list_items = Vec::<Line>::new();

    for message in app.messages.lock().unwrap().messages.iter() {
        list_items.push(Line::from_iter([
            Span::styled(
                format!("{:?}: ", message.role),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("{}", message.content.text_as_str().unwrap_or_default()),
                Style::default().fg(Color::Gray),
            ),
        ]))
    }

    let list = Paragraph::new(list_items.clone())
        .scroll((app.vertical_scroll as u16, 0))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1)),
        )
        .wrap(Wrap { trim: true });
    app.vertical_scroll_state = app.vertical_scroll_state.content_length(list_items.len());
    frame.render_widget(list, chunk[1]);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        chunk[1],
        &mut app.vertical_scroll_state,
    );

    if app.current_screen == CurrentScreen::Settings {
        let popup_block = Block::default()
            .title("Settings")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(popup_block, area);

        let popup_chunk = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let model_list = List::new(app.models.clone())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::Magenta))
            .highlight_symbol(">>");

        frame.render_stateful_widget(model_list, popup_chunk[0], &mut app.model_state);
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let input = Paragraph::new(Text::styled(
        &app.current_message,
        Style::default().fg(Color::Gray),
    ))
    .block(input_block);

    frame.render_widget(input, chunk[2]);

    frame.set_cursor_position(Position {
        x: chunk[2].x + app.character_position as u16 + 1,
        y: chunk[2].y + 1,
    });
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
