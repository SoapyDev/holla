use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use ratatui::widgets::{ListState, ScrollbarState};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CurrentScreen {
    #[default]
    Home,
    Settings,
    History,
    Exiting,
}

impl CurrentScreen {
    pub fn next(&mut self) {
        *self = match self {
            CurrentScreen::Home => CurrentScreen::Settings,
            CurrentScreen::Settings => CurrentScreen::History,
            CurrentScreen::History => CurrentScreen::Home,
            _ => CurrentScreen::Home,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct App {
    client: Client,
    pub current_message: String,
    pub character_position: usize,
    pub is_waiting: Arc<AtomicBool>,
    pub messages: Arc<Mutex<ChatRequest>>,
    pub models: Vec<String>,
    pub model_state: ListState,
    pub current_screen: CurrentScreen,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub viewport_height: usize,
}

const MODEL: &str = "mistral-nemo";

pub enum HollaCommand {
    Exit,
}

impl Display for HollaCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HollaCommand::Exit => write!(f, "/exit"),
        }
    }
}

impl App {
    pub async fn new() -> Self {
        let mut app = Self::default();
        app.load_models().await;
        app.load_history();
        app
    }

    pub fn with_viewport_height(self, height: usize) -> Self {
        Self {
            viewport_height: height,
            ..self
        }
    }

    async fn load_models(&mut self) {
        let adapter_kind = self.client.resolve_model_iden(MODEL).unwrap().adapter_kind;
        let models = self.client.all_model_names(adapter_kind).await.unwrap();
        self.models = models;
        self.model_state.select(Some(0));
    }

    fn load_history(&mut self) {
        #[cfg(target_os = "linux")]
        let home_dir = std::env::var("HOME").unwrap();

        #[cfg(target_os = "windows")]
        let home_dir = std::env::var("USERPROFILE").unwrap();

        let history_dir = format!("{}/.hollama/", home_dir);

        if !std::path::Path::new(&history_dir).exists() {
            return;
        }

        let history = std::fs::read_to_string(history_dir + "history.json").unwrap();
        let history: History = serde_json::from_str(&history).unwrap();
        *self.messages.lock().unwrap() = history.into();
    }

    fn parse_message(&mut self) {
        let message = self.current_message.to_lowercase();

        if message.starts_with(HollaCommand::Exit.to_string().as_str()) {
            self.exit();
        } else {
            self.save_message();
            self.execute_llm_query();
        }
    }
    pub fn save_message(&mut self) {
        let is_waiting = Arc::clone(&self.is_waiting);
        is_waiting.store(true, Ordering::Relaxed);

        let new_message = ChatMessage::user(self.current_message.to_owned());
        self.current_message = "".to_string();

        let message_locked = Arc::clone(&self.messages);

        let mut message = message_locked.lock().unwrap();
        *message = message.clone().append_message(new_message);
    }

    fn execute_llm_query(&mut self) {
        let is_waiting = Arc::clone(&self.is_waiting);
        let client = self.client.clone();

        let messages_cloned = Arc::clone(&self.messages);
        let model = self.models[self.model_state.selected().unwrap()].clone();
        tokio::spawn(async move {
            let mut message = messages_cloned.lock().unwrap().clone();
            let res = Self::send_message(client, &mut message, &model).await;
            *messages_cloned.lock().unwrap() = res;
            is_waiting.store(false, Ordering::Relaxed);
        });
    }
    async fn send_message(client: Client, messages: &mut ChatRequest, model: &str) -> ChatRequest {
        let model = if model.is_empty() { MODEL } else { model };

        let chat_res = client
            .exec_chat(model, messages.clone(), None)
            .await
            .unwrap();
        messages.clone().append_message(ChatMessage::assistant(
            chat_res.content_text_as_str().unwrap_or_default(),
        ))
    }
}

// Commands
impl App {
    pub(crate) fn remove_previous(&mut self) {
        let current = self.character_position;
        if current > 0 {
            let previous = current - 1;
            let before_char = self.current_message.chars().take(previous);
            let after_char = self.current_message.chars().skip(current);

            self.current_message = before_char.chain(after_char).collect();
            self.character_position = self.character_position.saturating_sub(1);
        }
    }

    pub(crate) fn remove_next(&mut self) {
        let current = self.character_position;
        if current < self.current_message.len() {
            let after_char = self.current_message.chars().skip(current + 1);
            self.current_message = self
                .current_message
                .chars()
                .take(current)
                .chain(after_char)
                .collect();
        }
    }

    pub(crate) fn handle_enter(&mut self) {
        self.parse_message();
        self.character_position = 0;
        if self.viewport_height < self.messages.lock().unwrap().messages.len() {
            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
            self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
        }
    }

    pub(crate) fn cursor_left(&mut self) {
        self.character_position = self.character_position.saturating_sub(1);
    }

    pub(crate) fn cursor_right(&mut self) {
        let limit = self.current_message.len();
        if self.character_position < limit {
            self.character_position = self.character_position.saturating_add(1);
        }
    }

    pub(crate) fn scroll_up(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn scroll_down(&mut self) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) fn insert_char(&mut self, key: char) {
        self.current_message.insert(self.character_position, key);
        self.character_position = self.character_position.saturating_add(1);
    }

    pub(crate) fn exit(&mut self) {
        #[cfg(target_os = "linux")]
        let home_dir = std::env::var("HOME").unwrap();

        #[cfg(target_os = "windows")]
        let home_dir = std::env::var("USERPROFILE").unwrap();

        create_history_directory(&home_dir).expect("Failed to create history directory");
        save_history_file(&home_dir, &self.messages.lock().unwrap())
            .expect("Failed to create history files");

        self.current_screen = CurrentScreen::Exiting;
    }
}

fn create_history_directory(home_dir: &str) -> io::Result<()> {
    let history_dir = format!("{}/.hollama/", home_dir);

    if std::path::Path::new(&history_dir).exists() {
        return Ok(());
    }

    std::fs::create_dir_all(history_dir)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct History {
    system: Option<String>,
    messages: Vec<HistoryMessage>,
}

impl<'a> From<&'a MutexGuard<'a, ChatRequest>> for History {
    fn from(msg: &MutexGuard<ChatRequest>) -> Self {
        History {
            system: msg.system.clone(),
            messages: msg
                .messages
                .iter()
                .map(|msg| HistoryMessage {
                    role: format!("{:?}", msg.role),
                    content: msg.content.text_as_str().unwrap_or_default().to_string(),
                })
                .collect::<Vec<_>>(),
        }
    }
}

impl Into<ChatRequest> for History {
    fn into(self) -> ChatRequest {
        ChatRequest {
            system: self.system,
            messages: self
                .messages
                .iter()
                .map(|msg| {
                    if msg.role == "system" {
                        ChatMessage::system(msg.content.clone())
                    } else {
                        ChatMessage::user(msg.content.clone())
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryMessage {
    role: String,
    content: String,
}

fn save_history_file(home_dir: &str, messages: &MutexGuard<ChatRequest>) -> io::Result<()> {
    let history_dir = format!("{}/.hollama/", home_dir);
    let history_file = format!("{}/history.json", history_dir);

    let history = History::from(messages);
    let json = serde_json::to_string(&history)?;

    std::fs::write(history_file, json)?;

    Ok(())
}
