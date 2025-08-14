use super::events::{EventHandler, TUIEvent};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc;

type Backend = CrosstermBackend<Stdout>;

/// Сообщение в чате
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Состояние чата
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub input_cursor_position: usize,
    pub scroll_offset: usize,
    pub is_processing: bool,
    pub should_quit: bool,
    pub multiline_mode: bool,
    pub status_message: String,
}

impl Default for ChatState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatState {
    pub fn new() -> Self {
        let welcome_message = ChatMessage {
            role: MessageRole::System,
            content: "Добро пожаловать в MAGRAY CLI! Введите сообщение и нажмите Enter для отправки. Ctrl+Enter для многострочного ввода, Ctrl+D для выхода.".to_string(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        };

        Self {
            messages: vec![welcome_message],
            input_buffer: String::new(),
            input_cursor_position: 0,
            scroll_offset: 0,
            is_processing: false,
            should_quit: false,
            multiline_mode: false,
            status_message: "Готов к работе".to_string(),
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) {
        let message = ChatMessage {
            role,
            content,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        };
        self.messages.push(message);
    }

    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.input_cursor_position = 0;
        self.multiline_mode = false;
    }
}

/// TUI приложение для чата
pub struct ChatTUI {
    terminal: Terminal<Backend>,
    event_handler: EventHandler,
    pub state: ChatState,
    response_receiver: Option<mpsc::UnboundedReceiver<String>>,
}

impl ChatTUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let event_handler = EventHandler::new(Duration::from_millis(100));
        let state = ChatState::new();

        Ok(ChatTUI {
            terminal,
            event_handler,
            state,
            response_receiver: None,
        })
    }

    pub fn set_response_receiver(&mut self, receiver: mpsc::UnboundedReceiver<String>) {
        self.response_receiver = Some(receiver);
    }

    pub async fn run<F>(&mut self, message_handler: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(String) -> Option<mpsc::UnboundedReceiver<String>>,
    {
        while !self.state.should_quit {
            // Отрисовка UI
            self.terminal
                .draw(|f| Self::render_ui(f, &mut self.state))?;

            // Проверка ответов от AI
            if let Some(receiver) = &mut self.response_receiver {
                if let Ok(response) = receiver.try_recv() {
                    self.state.add_message(MessageRole::Assistant, response);
                    self.state.is_processing = false;
                    self.state.status_message = "Готов к работе".to_string();
                }
            }

            // Обработка событий
            if let Ok(event) = self.event_handler.next() {
                match event {
                    TUIEvent::Key(key) => self.handle_key_event(key, &message_handler)?,
                    TUIEvent::Resize(_, _) => {}
                    TUIEvent::Tick => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_key_event<F>(
        &mut self,
        key: KeyEvent,
        message_handler: &F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(String) -> Option<mpsc::UnboundedReceiver<String>>,
    {
        if self.state.is_processing {
            // Во время обработки разрешаем только Ctrl+C для отмены
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.state.is_processing = false;
                self.state.status_message = "Отменено".to_string();
            }
            return Ok(());
        }

        match key.code {
            // Выход
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.should_quit = true;
            }
            // Многострочный режим
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.input_buffer.push('\n');
                self.state.input_cursor_position = self.state.input_buffer.len();
                self.state.multiline_mode = true;
            }
            // Отправка сообщения
            KeyCode::Enter if !self.state.multiline_mode => {
                if !self.state.input_buffer.trim().is_empty() {
                    let message = self.state.input_buffer.clone();
                    self.state.add_message(MessageRole::User, message.clone());
                    self.state.clear_input();
                    self.state.is_processing = true;
                    self.state.status_message = "Обработка...".to_string();

                    // Отправка сообщения обработчику
                    if let Some(receiver) = message_handler(message) {
                        self.response_receiver = Some(receiver);
                    }
                }
            }
            // Отправка в многострочном режиме
            KeyCode::Enter
                if self.state.multiline_mode && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                if !self.state.input_buffer.trim().is_empty() {
                    let message = self.state.input_buffer.clone();
                    self.state.add_message(MessageRole::User, message.clone());
                    self.state.clear_input();
                    self.state.is_processing = true;
                    self.state.status_message = "Обработка...".to_string();

                    // Отправка сообщения обработчику
                    if let Some(receiver) = message_handler(message) {
                        self.response_receiver = Some(receiver);
                    }
                }
            }
            // Навигация в тексте
            KeyCode::Left => {
                if self.state.input_cursor_position > 0 {
                    self.state.input_cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.state.input_cursor_position < self.state.input_buffer.len() {
                    self.state.input_cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.state.input_cursor_position = 0;
            }
            KeyCode::End => {
                self.state.input_cursor_position = self.state.input_buffer.len();
            }
            // Удаление символов
            KeyCode::Backspace => {
                if self.state.input_cursor_position > 0 {
                    self.state
                        .input_buffer
                        .remove(self.state.input_cursor_position - 1);
                    self.state.input_cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.state.input_cursor_position < self.state.input_buffer.len() {
                    self.state
                        .input_buffer
                        .remove(self.state.input_cursor_position);
                }
            }
            // Прокрутка истории
            KeyCode::PageUp => {
                if self.state.scroll_offset > 0 {
                    self.state.scroll_offset = self.state.scroll_offset.saturating_sub(5);
                }
            }
            KeyCode::PageDown => {
                self.state.scroll_offset = self.state.scroll_offset.saturating_add(5);
            }
            // Ввод символов
            KeyCode::Char(c) => {
                self.state
                    .input_buffer
                    .insert(self.state.input_cursor_position, c);
                self.state.input_cursor_position += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn render_ui(f: &mut Frame, state: &mut ChatState) {
        let size = f.area();

        // Главный layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Заголовок
                    Constraint::Min(1),    // История чата
                    Constraint::Length(5), // Поле ввода
                    Constraint::Length(1), // Статус бар
                ]
                .as_ref(),
            )
            .split(size);

        // Заголовок
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                "MAGRAY CLI",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("Chat Mode", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled("Ctrl+D to exit", Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        // История чата
        Self::render_chat_history(f, chunks[1], state);

        // Поле ввода
        Self::render_input_field(f, chunks[2], state);

        // Статус бар
        Self::render_status_bar(f, chunks[3], state);
    }

    fn render_chat_history(f: &mut Frame, area: Rect, state: &ChatState) {
        let messages: Vec<ListItem> = state
            .messages
            .iter()
            .flat_map(|msg| {
                let style = match msg.role {
                    MessageRole::User => Style::default().fg(Color::Cyan),
                    MessageRole::Assistant => Style::default().fg(Color::Green),
                    MessageRole::System => Style::default().fg(Color::Yellow),
                };

                let role_label = match msg.role {
                    MessageRole::User => "You",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };

                let mut lines = vec![];

                // Заголовок сообщения
                lines.push(ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("[{}] ", msg.timestamp),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("{role_label}: "),
                        style.add_modifier(Modifier::BOLD),
                    ),
                ])));

                // Контент сообщения
                for line in msg.content.lines() {
                    lines.push(ListItem::new(Line::from(Span::styled(
                        format!("  {line}"),
                        style,
                    ))));
                }

                // Пустая строка между сообщениями
                lines.push(ListItem::new(Line::from("")));

                lines
            })
            .collect();

        let messages_list =
            List::new(messages).block(Block::default().borders(Borders::ALL).title("Chat History"));

        f.render_widget(messages_list, area);
    }

    fn render_input_field(f: &mut Frame, area: Rect, state: &ChatState) {
        let input_mode = if state.multiline_mode {
            "Multiline (Shift+Enter to send)"
        } else {
            "Single line (Enter to send, Ctrl+Enter for multiline)"
        };

        let input = Paragraph::new(state.input_buffer.as_str())
            .style(if state.is_processing {
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::White)
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Input - {input_mode}"))
                    .border_style(if state.is_processing {
                        Style::default().fg(Color::Gray)
                    } else {
                        Style::default().fg(Color::Cyan)
                    }),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(input, area);

        // Курсор
        if !state.is_processing {
            let cursor_x = area.x + 1 + state.input_cursor_position as u16 % (area.width - 2);
            let cursor_y = area.y + 1 + state.input_cursor_position as u16 / (area.width - 2);
            f.set_cursor_position((
                cursor_x.min(area.x + area.width - 2),
                cursor_y.min(area.y + area.height - 2),
            ));
        }
    }

    fn render_status_bar(f: &mut Frame, area: Rect, state: &ChatState) {
        let status = Line::from(vec![
            Span::styled(
                format!("Messages: {} ", state.messages.len()),
                Style::default().fg(Color::Gray),
            ),
            Span::raw("| "),
            Span::styled(
                state.status_message.as_str(),
                if state.is_processing {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
            Span::raw(" | "),
            Span::styled(
                if state.is_processing {
                    "Processing... (Ctrl+C to cancel)"
                } else {
                    "Ready"
                },
                Style::default().fg(Color::Gray),
            ),
        ]);

        let status_bar = Paragraph::new(status).style(Style::default().bg(Color::Black));
        f.render_widget(status_bar, area);
    }
}

impl Drop for ChatTUI {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    }
}
