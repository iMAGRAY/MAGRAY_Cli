use crate::services;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
use std::io;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::{timeout, Duration as TokioDuration};

/// Сообщение в чате
#[derive(Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: String,
}

/// Состояние TUI чата
struct TuiChatState {
    messages: Vec<ChatMessage>,
    input: String,
    cursor_position: usize,
    scroll_offset: usize,
    is_processing: bool,
}

impl TuiChatState {
    fn new() -> Self {
        let mut state = Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            is_processing: false,
        };

        state.add_message(
            "System".to_string(),
            "🎉 Добро пожаловать в MAGRAY CLI TUI Chat! \n\n✨ Это полноценный чат интерфейс как в Claude Code!\n\n💡 Возможности:\n• Отправка сообщений: Enter\n• Прокрутка истории: Page Up/Down\n• Навигация: стрелки\n• Выход: ESC или Ctrl+C\n\n🤖 Напишите ваше сообщение ниже:".to_string(),
        );

        state
    }

    fn add_message(&mut self, role: String, content: String) {
        self.messages.push(ChatMessage {
            role,
            content,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        });
    }

    /// Converts character position to byte index for UTF-8 string operations
    fn char_to_byte_index(&self, char_pos: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_pos)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.input.len())
    }

    /// Converts byte index to character position for cursor positioning
    #[allow(dead_code)]
    fn byte_to_char_index(&self, byte_idx: usize) -> usize {
        self.input[..byte_idx.min(self.input.len())].chars().count()
    }

    /// Get the maximum cursor position (character count)
    fn max_cursor_position(&self) -> usize {
        self.input.chars().count()
    }

    /// Safely insert character at cursor position
    fn insert_char(&mut self, c: char) {
        let byte_idx = self.char_to_byte_index(self.cursor_position);
        self.input.insert(byte_idx, c);
        self.cursor_position += 1;
    }

    /// Safely remove character before cursor position
    fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_idx = self.char_to_byte_index(self.cursor_position);
            if let Some(ch) = self.input.chars().nth(self.cursor_position) {
                let char_len = ch.len_utf8();
                for _ in 0..char_len {
                    if byte_idx < self.input.len() {
                        self.input.remove(byte_idx);
                    }
                }
            }
        }
    }

    /// Move cursor left by one character
    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right by one character
    fn move_cursor_right(&mut self) {
        let max_pos = self.max_cursor_position();
        if self.cursor_position < max_pos {
            self.cursor_position += 1;
        }
    }
}

/// Запускает TUI чат
pub async fn run_tui_chat(service: &crate::services::OrchestrationService) -> Result<()> {
    println!("🚀 Инициализация TUI чат интерфейса...");
    println!("📱 Создаём полноценный чат как в Claude Code...");

    // Быстрая настройка терминала с улучшенной обработкой ошибок
    enable_raw_mode()
        .map_err(|e| anyhow::format_err!("Не удалось включить raw mode терминала: {}", e))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| anyhow::format_err!("Не удалось настроить терминал: {}", e))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| anyhow::format_err!("Не удалось создать терминал: {}", e))?;

    // Состояние приложения с оптимизированной инициализацией
    let mut state = TuiChatState::new();

    // Главный цикл
    loop {
        // Отрисовка UI
        if let Err(e) = terminal.draw(|f| render_ui(f, &state)) {
            return Err(anyhow::format_err!("Terminal draw failed: {}", e));
        }

        // Обработка событий
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Выход (Ctrl+C или ESC)
                if (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
                    || key.code == KeyCode::Esc
                {
                    break;
                }

                if !state.is_processing {
                    match key.code {
                        // Отправка сообщения
                        KeyCode::Enter => {
                            if !state.input.trim().is_empty() {
                                let message = state.input.clone();
                                state.add_message("You".to_string(), message.clone());
                                state.input.clear();
                                state.cursor_position = 0;
                                state.is_processing = true;

                                // Отрисовка с индикатором обработки
                                terminal.draw(|f| render_ui(f, &state))?;

                                // Получение ответа с timeout защитой
                                let request_future = service.process_user_request(&message);
                                match timeout(TokioDuration::from_secs(60), request_future).await {
                                    Ok(Ok(response)) => {
                                        state.add_message("Assistant".to_string(), response);
                                    }
                                    Ok(Err(e)) => {
                                        state.add_message(
                                            "Error".to_string(),
                                            format!("Ошибка: {e}"),
                                        );
                                    }
                                    Err(_) => {
                                        state.add_message(
                                            "Error".to_string(),
                                            "Timeout: Запрос занял более 60 секунд".to_string(),
                                        );
                                    }
                                }

                                state.is_processing = false;
                            }
                        }
                        // Навигация
                        KeyCode::Left => {
                            if state.cursor_position > 0 {
                                state.cursor_position -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if state.cursor_position < state.input.len() {
                                state.cursor_position += 1;
                            }
                        }
                        KeyCode::Home => {
                            state.cursor_position = 0;
                        }
                        KeyCode::End => {
                            state.cursor_position = state.input.len();
                        }
                        // Редактирование
                        KeyCode::Backspace => {
                            if state.cursor_position > 0 {
                                state.input.remove(state.cursor_position - 1);
                                state.cursor_position -= 1;
                            }
                        }
                        KeyCode::Delete => {
                            if state.cursor_position < state.input.len() {
                                state.input.remove(state.cursor_position);
                            }
                        }
                        // Прокрутка
                        KeyCode::PageUp => {
                            state.scroll_offset = state.scroll_offset.saturating_sub(5);
                        }
                        KeyCode::PageDown => {
                            state.scroll_offset = state.scroll_offset.saturating_add(5);
                        }
                        // Ввод символов
                        KeyCode::Char(c) => {
                            state.input.insert(state.cursor_position, c);
                            state.cursor_position += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Восстановление терминала
    let cleanup_result = (|| -> Result<()> {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    })();

    if let Err(e) = cleanup_result {
        eprintln!("⚠️  Warning: Terminal cleanup error: {e}");
        // Попытка force cleanup
        let _ = disable_raw_mode();
    }

    Ok(())
}

/// Отрисовка UI
fn render_ui(f: &mut Frame, state: &TuiChatState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Заголовок
            Constraint::Min(1),    // Чат
            Constraint::Length(3), // Ввод
            Constraint::Length(1), // Статус
        ])
        .split(f.area());

    // Заголовок
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "🤖 MAGRAY CLI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            "💬 Claude Code-Style Chat",
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled("ESC or Ctrl+C to exit", Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // История чата
    render_chat_history(f, chunks[1], state);

    // Поле ввода
    render_input_field(f, chunks[2], state);

    // Статус бар
    render_status_bar(f, chunks[3], state);
}

/// Отрисовка истории чата
fn render_chat_history(f: &mut Frame, area: Rect, state: &TuiChatState) {
    let messages: Vec<ListItem> = state
        .messages
        .iter()
        .skip(state.scroll_offset)
        .flat_map(|msg| {
            let style = match msg.role.as_str() {
                "You" => Style::default().fg(Color::Cyan),
                "Assistant" => Style::default().fg(Color::Green),
                "System" => Style::default().fg(Color::Yellow),
                "Error" => Style::default().fg(Color::Red),
                _ => Style::default(),
            };

            let mut lines = vec![];

            // Заголовок сообщения
            lines.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", msg.timestamp),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{}: ", msg.role),
                    style.add_modifier(Modifier::BOLD),
                ),
            ])));

            // Контент - разбиваем длинные строки
            let max_width = (area.width - 4) as usize;
            for line in msg.content.lines() {
                if line.len() <= max_width {
                    lines.push(ListItem::new(Line::from(Span::styled(
                        format!("  {line}"),
                        style,
                    ))));
                } else {
                    // Разбиваем длинную строку
                    for chunk in line.chars().collect::<Vec<char>>().chunks(max_width) {
                        lines.push(ListItem::new(Line::from(Span::styled(
                            format!("  {}", chunk.iter().collect::<String>()),
                            style,
                        ))));
                    }
                }
            }

            // Пустая строка между сообщениями
            lines.push(ListItem::new(Line::from("")));

            lines
        })
        .collect();

    let messages_list = List::new(messages).block(
        Block::default()
            .borders(Borders::ALL)
            .title("💬 Chat History (Claude Code Style)"),
    );

    f.render_widget(messages_list, area);
}

/// Отрисовка поля ввода
fn render_input_field(f: &mut Frame, area: Rect, state: &TuiChatState) {
    let input = Paragraph::new(state.input.as_str())
        .style(if state.is_processing {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(Color::White)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(if state.is_processing {
                    "🔄 AI Processing..."
                } else {
                    "✍️ Your Message (Enter to send)"
                })
                .border_style(if state.is_processing {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::Cyan)
                }),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(input, area);

    // Курсор
    if !state.is_processing && area.width > 2 {
        let cursor_x = area.x + 1 + (state.cursor_position as u16).min(area.width - 2);
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Отрисовка статус бара
fn render_status_bar(f: &mut Frame, area: Rect, state: &TuiChatState) {
    let status = Line::from(vec![
        Span::styled(
            format!("Messages: {} ", state.messages.len()),
            Style::default().fg(Color::Gray),
        ),
        Span::raw("| "),
        if state.is_processing {
            Span::styled("Processing...", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("Ready", Style::default().fg(Color::Green))
        },
        Span::raw(" | "),
        Span::styled(
            "↑↓ scroll | Enter send | ESC/Ctrl+C exit",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_bar = Paragraph::new(status);
    f.render_widget(status_bar, area);
}

/// Структура для асинхронной инициализации сервиса
struct AsyncServiceState {
    service: Arc<RwLock<Option<Arc<services::OrchestrationService>>>>,
    is_initializing: Arc<RwLock<bool>>,
    init_error: Arc<RwLock<Option<String>>>,
}

impl AsyncServiceState {
    fn new() -> Self {
        Self {
            service: Arc::new(RwLock::new(None)),
            is_initializing: Arc::new(RwLock::new(true)),
            init_error: Arc::new(RwLock::new(None)),
        }
    }

    async fn start_initialization(&self) {
        let service_clone = Arc::clone(&self.service);
        let is_initializing_clone = Arc::clone(&self.is_initializing);
        let init_error_clone = Arc::clone(&self.init_error);

        tokio::spawn(async move {
            // Попробуем создать OrchestrationService с коротким таймаутом
            let service_future = crate::create_orchestrator_service();
            match timeout(TokioDuration::from_secs(10), service_future).await {
                Ok(Ok(service)) => {
                    if let Ok(mut service_lock) = service_clone.write() {
                        *service_lock = Some(Arc::new(service));
                    }
                }
                Ok(Err(e)) => {
                    // Используем fallback
                    match services::OrchestrationService::with_llm_fallback().await {
                        Ok(fallback_service) => {
                            if let Ok(mut service_lock) = service_clone.write() {
                                *service_lock = Some(Arc::new(fallback_service));
                            }
                        }
                        Err(fallback_err) => {
                            if let Ok(mut error_lock) = init_error_clone.write() {
                                *error_lock = Some(format!(
                                    "Failed to initialize: {e} (fallback: {fallback_err})"
                                ));
                            }
                        }
                    }
                }
                Err(_) => {
                    // Таймаут - используем fallback
                    match services::OrchestrationService::with_llm_fallback().await {
                        Ok(fallback_service) => {
                            if let Ok(mut service_lock) = service_clone.write() {
                                *service_lock = Some(Arc::new(fallback_service));
                            }
                        }
                        Err(fallback_err) => {
                            if let Ok(mut error_lock) = init_error_clone.write() {
                                *error_lock = Some(format!(
                                    "Initialization timeout (fallback: {fallback_err})"
                                ));
                            }
                        }
                    }
                }
            }

            // Отмечаем что инициализация завершена
            if let Ok(mut init_lock) = is_initializing_clone.write() {
                *init_lock = false;
            }
        });
    }
}

/// Запуск TUI чата с асинхронной инициализацией сервиса
pub async fn run_tui_chat_with_async_init() -> Result<()> {
    // Очистим терминал перед инициализацией TUI
    print!("\x1b[2J\x1b[H"); // Clear screen and move cursor to home
    std::io::Write::flush(&mut std::io::stdout()).ok();

    // Настройка терминала
    enable_raw_mode().map_err(|e| anyhow::anyhow!("Failed to enable raw mode: {}", e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| anyhow::anyhow!("Failed to setup terminal: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| anyhow::anyhow!("Failed to create terminal: {}", e))?;

    // Состояние чата
    let mut state = TuiChatState::new();
    state.add_message(
        "System".to_string(),
        "🚀 Инициализация MAGRAY AI системы...\n⏳ Подключение к мульти-агентной архитектуре..."
            .to_string(),
    );

    // Асинхронная инициализация сервиса
    let async_state = AsyncServiceState::new();
    async_state.start_initialization().await;

    let result = run_tui_loop(&mut terminal, &mut state, &async_state).await;

    // Восстановление терминала
    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .ok();

    result
}

/// Основной цикл TUI с асинхронным сервисом
async fn run_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiChatState,
    async_state: &AsyncServiceState,
) -> Result<()> {
    let mut service_ready = false;
    let mut last_init_check = std::time::Instant::now();

    loop {
        // Проверяем состояние инициализации каждые 500ms
        if !service_ready && last_init_check.elapsed() >= std::time::Duration::from_millis(500) {
            last_init_check = std::time::Instant::now();

            if let Ok(is_initializing) = async_state.is_initializing.read() {
                if !*is_initializing {
                    if let Ok(error) = async_state.init_error.read() {
                        if let Some(err_msg) = &*error {
                            state.add_message(
                                "System".to_string(),
                                format!(
                                    "❌ Ошибка инициализации: {err_msg}\n💡 Используйте простые команды"
                                ),
                            );
                        } else {
                            state.add_message(
                                "System".to_string(),
                                "✅ AI система готова к работе!\n💬 Напишите ваше сообщение ниже:"
                                    .to_string(),
                            );
                        }
                    }
                    service_ready = true;
                }
            }
        }

        // Отрисовка интерфейса
        terminal.draw(|f| {
            render_ui(f, state);
        })?;

        // Обработка событий
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
                    || key.code == KeyCode::Esc
                {
                    break;
                }

                if !state.is_processing {
                    match key.code {
                        KeyCode::Enter => {
                            if !state.input.trim().is_empty() {
                                let message = state.input.clone();
                                state.add_message("You".to_string(), message.clone());
                                state.input.clear();
                                state.cursor_position = 0;
                                state.is_processing = true;

                                // Обработка сообщения
                                if let Ok(service_lock) = async_state.service.read() {
                                    if let Some(service) = &*service_lock {
                                        let service_clone = Arc::clone(service);
                                        let response =
                                            process_message_async(service_clone, message).await;
                                        state.add_message("AI".to_string(), response);
                                    } else {
                                        state.add_message(
                                            "AI".to_string(),
                                            "Система пока инициализируется. Попробуйте через несколько секунд.".to_string(),
                                        );
                                    }
                                }
                                state.is_processing = false;
                            }
                        }
                        KeyCode::Char(c) => {
                            // Use UTF-8 safe character insertion
                            state.insert_char(c);
                        }
                        KeyCode::Backspace => {
                            // Use UTF-8 safe backspace operation
                            state.backspace();
                        }
                        KeyCode::Left => {
                            // Move cursor left by one character (UTF-8 safe)
                            state.move_cursor_left();
                        }
                        KeyCode::Right => {
                            // Move cursor right by one character (UTF-8 safe)
                            state.move_cursor_right();
                        }
                        KeyCode::PageUp => {
                            if state.scroll_offset > 0 {
                                state.scroll_offset -= 1;
                            }
                        }
                        KeyCode::PageDown => {
                            if state.scroll_offset < state.messages.len().saturating_sub(1) {
                                state.scroll_offset += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

/// Асинхронная обработка сообщения для TUI (без workflow логов)
async fn process_message_async(
    service: Arc<services::OrchestrationService>,
    message: String,
) -> String {
    match timeout(
        TokioDuration::from_secs(30),
        service.process_tui_message(&message),
    )
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => format!("Ошибка обработки: {e}"),
        Err(_) => "Таймаут при обработке запроса (30 сек)".to_string(),
    }
}
