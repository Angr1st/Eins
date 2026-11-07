use std::collections::HashMap;
use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eins_lib::api::{
    GameSetupResponse, GameSetupState, GameSetupStateResponse, JoinResponseState, ServerEvent,
};
use eins_lib::cards::{CardReference, CardTypes, Color};
use eins_lib::game::GameView;
use eins_lib::infrastructure::Player;
use ratatui::layout::{Position, Rect};
use uuid::Uuid;

use crate::api::{Api, Notice};

const OFFLINE: &str = "Lost the connection to the server, reconnecting…";

pub enum Screen {
    Register {
        nick: String,
    },
    Lobby {
        setups: Vec<GameSetupResponse>,
        selected: usize,
    },
    Waiting {
        setup: Option<GameSetupStateResponse>,
    },
    Game {
        view: Option<GameView>,
        selected: usize,
        /// The "Eins" button: sent with the next play, then reset.
        eins: bool,
        /// A wild card waiting for its color.
        picking_color: Option<CardReference>,
    },
}

pub enum PromptKind {
    PrivateGameCode,
    JoinGameCode(Uuid),
}

pub struct Prompt {
    pub kind: PromptKind,
    pub text: String,
}

#[derive(Clone, Copy)]
pub enum Action {
    PlayCard(usize),
    Draw,
    ToggleEins,
    PickColor(Color),
    LeaveGame,
    JoinSetup(usize),
    CreateOpen,
    CreatePrivate,
    Start,
    LeaveSetup,
}

pub struct App {
    api: Api,
    pub me: Option<Player>,
    pub nicks: HashMap<Uuid, String>,
    pub screen: Screen,
    pub prompt: Option<Prompt>,
    pub status: String,
    pub quit: bool,
    /// Areas that react to a mouse click, collected while drawing.
    pub clickables: Vec<(Rect, Action)>,
    notices: Sender<Notice>,
}

impl App {
    pub fn new(api: Api, notices: Sender<Notice>) -> Self {
        Self {
            api,
            me: None,
            nicks: HashMap::new(),
            screen: Screen::Register {
                nick: String::new(),
            },
            prompt: None,
            status: String::new(),
            quit: false,
            clickables: vec![],
            notices,
        }
    }

    pub fn nick(&self, id: &Uuid) -> String {
        self.nicks
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string()[..8].to_string())
    }

    pub fn my_id(&self) -> Option<Uuid> {
        self.me.as_ref().map(Player::get_id)
    }

    /// For background refreshes: shows an error but keeps the message of the last action.
    fn fetched<T>(&mut self, result: Result<T, String>) -> Option<T> {
        result.map_err(|error| self.status = error).ok()
    }

    /// For the player's own actions: shows an error, or clears the last message.
    fn report<T>(&mut self, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => {
                self.status.clear();
                Some(value)
            }
            Err(error) => {
                self.status = error;
                None
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if let Some(prompt) = &mut self.prompt {
            match key.code {
                KeyCode::Char(c) => prompt.text.push(c),
                KeyCode::Backspace => {
                    prompt.text.pop();
                }
                KeyCode::Esc => self.prompt = None,
                KeyCode::Enter => {
                    let prompt = self.prompt.take().expect("checked above");
                    self.submit_prompt(prompt);
                }
                _ => {}
            }
            return;
        }

        let action = match &mut self.screen {
            Screen::Register { nick } => {
                match key.code {
                    KeyCode::Char(c) => nick.push(c),
                    KeyCode::Backspace => {
                        nick.pop();
                    }
                    KeyCode::Enter => self.register(),
                    KeyCode::Esc => self.quit = true,
                    _ => {}
                }
                None
            }
            Screen::Lobby { setups, selected } => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = selected.saturating_sub(1);
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1).min(setups.len().saturating_sub(1));
                    None
                }
                KeyCode::Enter => Some(Action::JoinSetup(*selected)),
                KeyCode::Char('n') => Some(Action::CreateOpen),
                KeyCode::Char('p') => Some(Action::CreatePrivate),
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.quit = true;
                    None
                }
                _ => None,
            },
            Screen::Waiting { .. } => match key.code {
                KeyCode::Char('s') => Some(Action::Start),
                KeyCode::Char('l') => Some(Action::LeaveSetup),
                KeyCode::Char('q') => {
                    self.quit = true;
                    None
                }
                _ => None,
            },
            Screen::Game {
                view,
                selected,
                picking_color,
                ..
            } => {
                if picking_color.is_some() {
                    match key.code {
                        KeyCode::Char('1' | 'r') => Some(Action::PickColor(Color::Red)),
                        KeyCode::Char('2' | 'b') => Some(Action::PickColor(Color::Blue)),
                        KeyCode::Char('3' | 'o') => Some(Action::PickColor(Color::Orange)),
                        KeyCode::Char('4' | 'g') => Some(Action::PickColor(Color::Green)),
                        KeyCode::Esc => {
                            *picking_color = None;
                            None
                        }
                        _ => None,
                    }
                } else {
                    let hand_size = view.as_ref().map_or(0, |v| v.hand.len());
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            *selected = selected.saturating_sub(1);
                            None
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            *selected = (*selected + 1).min(hand_size.saturating_sub(1));
                            None
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::PlayCard(*selected)),
                        KeyCode::Char('d') => Some(Action::Draw),
                        KeyCode::Char('e') => Some(Action::ToggleEins),
                        KeyCode::Char('L') => Some(Action::LeaveGame),
                        KeyCode::Char('q') => {
                            self.quit = true;
                            None
                        }
                        _ => None,
                    }
                }
            }
        };
        if let Some(action) = action {
            self.act(action);
        }
    }

    pub fn on_click(&mut self, column: u16, row: u16) {
        let position = Position::new(column, row);
        let hit = self
            .clickables
            .iter()
            .rev()
            .find(|(area, _)| area.contains(position))
            .map(|(_, action)| *action);
        if let Some(action) = hit {
            if let (Action::PlayCard(index), Screen::Game { selected, .. }) =
                (action, &mut self.screen)
            {
                *selected = index;
            }
            self.act(action);
        }
    }

    pub fn act(&mut self, action: Action) {
        // The status line shows the outcome of the latest action only.
        self.status.clear();
        match action {
            Action::PlayCard(index) => {
                let Screen::Game {
                    view: Some(view),
                    picking_color,
                    ..
                } = &mut self.screen
                else {
                    return;
                };
                let Some(card) = view.hand.get(index) else {
                    return;
                };
                match card.card {
                    CardTypes::Wild(_) => *picking_color = Some(card.id),
                    CardTypes::Normal(_) => {
                        let id = card.id;
                        self.play(id, None);
                    }
                }
            }
            Action::PickColor(color) => {
                if let Screen::Game { picking_color, .. } = &mut self.screen {
                    if let Some(card) = picking_color.take() {
                        self.play(card, Some(color));
                    }
                }
            }
            Action::Draw => {
                let result = self.api.draw();
                if let Some(new_view) = self.report(result) {
                    self.show_view(new_view);
                }
            }
            Action::ToggleEins => {
                if let Screen::Game { eins, .. } = &mut self.screen {
                    *eins = !*eins;
                }
            }
            Action::LeaveGame => {
                let result = self.api.leave_game();
                if self.report(result).is_some() {
                    self.enter_lobby();
                }
            }
            Action::JoinSetup(index) => {
                let Screen::Lobby { setups, .. } = &self.screen else {
                    return;
                };
                let Some(setup) = setups.get(index) else {
                    return;
                };
                if setup.state == GameSetupState::Closed {
                    self.prompt = Some(Prompt {
                        kind: PromptKind::JoinGameCode(setup.id),
                        text: String::new(),
                    });
                } else {
                    let id = setup.id;
                    self.join(id, None);
                }
            }
            Action::CreateOpen => self.create(None),
            Action::CreatePrivate => {
                self.prompt = Some(Prompt {
                    kind: PromptKind::PrivateGameCode,
                    text: String::new(),
                });
            }
            Action::Start => {
                let result = self.api.start();
                if self.report(result).is_some() {
                    self.enter_game();
                }
            }
            Action::LeaveSetup => {
                let result = self.api.leave_setup();
                if self.report(result).is_some() {
                    self.enter_lobby();
                }
            }
        }
    }

    fn submit_prompt(&mut self, prompt: Prompt) {
        let code = Some(prompt.text).filter(|text| !text.trim().is_empty());
        match prompt.kind {
            PromptKind::PrivateGameCode => self.create(code),
            PromptKind::JoinGameCode(game_id) => self.join(game_id, code),
        }
    }

    fn register(&mut self) {
        let Screen::Register { nick } = &self.screen else {
            return;
        };
        let result = self.api.register(nick.trim().to_string());
        if let Some(player) = self.report(result) {
            self.me = Some(player);
            self.api.listen(self.notices.clone());
            self.enter_lobby();
        }
    }

    fn create(&mut self, code: Option<String>) {
        let result = self.api.create_setup(code);
        if self.report(result).is_some() {
            self.enter_waiting();
        }
    }

    fn join(&mut self, game_id: Uuid, code: Option<String>) {
        let result = self.api.join(game_id, code);
        match self.report(result).map(|r| r.state) {
            Some(JoinResponseState::Success) => self.enter_waiting(),
            Some(JoinResponseState::Failure) => {
                self.status = "Wrong game code, or the game is full".into();
            }
            None => {}
        }
    }

    fn play(&mut self, card: CardReference, color: Option<Color>) {
        let Screen::Game { eins, .. } = &self.screen else {
            return;
        };
        let result = self.api.play(card, color, *eins);
        if let Some(new_view) = self.report(result) {
            if let Screen::Game { eins, .. } = &mut self.screen {
                *eins = false;
            }
            self.show_view(new_view);
        }
    }

    fn show_view(&mut self, new_view: GameView) {
        if let Screen::Game { view, selected, .. } = &mut self.screen {
            *selected = (*selected).min(new_view.hand.len().saturating_sub(1));
            *view = Some(new_view);
        }
    }

    fn enter_lobby(&mut self) {
        self.screen = Screen::Lobby {
            setups: vec![],
            selected: 0,
        };
        self.refresh();
    }

    fn enter_waiting(&mut self) {
        self.screen = Screen::Waiting { setup: None };
        self.refresh();
    }

    fn enter_game(&mut self) {
        self.screen = Screen::Game {
            view: None,
            selected: 0,
            eins: false,
            picking_color: None,
        };
        self.refresh();
    }

    fn load_nicks(&mut self) {
        if let Ok(players) = self.api.players() {
            self.nicks = players.into_iter().map(|p| (p.id, p.nick)).collect();
        }
    }

    /// Reacts to the server's event stream by refreshing what the current screen shows.
    pub fn on_notice(&mut self, notice: Notice) {
        let refresh = match notice {
            Notice::Connected => {
                if self.status == OFFLINE {
                    self.status.clear();
                }
                // Changes may have happened while the stream was down.
                true
            }
            Notice::Disconnected => {
                self.status = OFFLINE.into();
                false
            }
            Notice::Event(ServerEvent::Resync) => true,
            Notice::Event(ServerEvent::Lobby) => {
                matches!(self.screen, Screen::Lobby { .. } | Screen::Waiting { .. })
            }
            Notice::Event(ServerEvent::Game { game_id }) => match &self.screen {
                Screen::Game { view, .. } => view.as_ref().is_none_or(|v| v.game_id == game_id),
                _ => false,
            },
        };
        if refresh {
            self.refresh();
        }
    }

    fn refresh(&mut self) {
        match &self.screen {
            Screen::Register { .. } => {}
            Screen::Lobby { .. } => {
                self.load_nicks();
                let result = self.api.setups();
                if let (Some(new_setups), Screen::Lobby { setups, selected }) =
                    (self.fetched(result), &mut self.screen)
                {
                    *selected = (*selected).min(new_setups.len().saturating_sub(1));
                    *setups = new_setups;
                }
            }
            Screen::Waiting { .. } => {
                let Some(me) = self.fetched(self.api.me()) else {
                    return;
                };
                let in_game = me.is_in_game();
                let free = me.is_free();
                self.me = Some(me);
                if in_game {
                    self.enter_game();
                } else if free {
                    self.enter_lobby();
                    self.status = "The game setup was closed".into();
                } else {
                    let result = self.api.my_setup();
                    if let (Some(new_setup), Screen::Waiting { setup }) =
                        (self.fetched(result), &mut self.screen)
                    {
                        *setup = Some(new_setup);
                    }
                }
            }
            Screen::Game { .. } => {
                match self.api.game() {
                    Ok(new_view) => {
                        let unknown = new_view
                            .players
                            .iter()
                            .any(|p| !self.nicks.contains_key(&p.id));
                        if unknown {
                            self.load_nicks();
                        }
                        self.show_view(new_view);
                    }
                    Err(error) => {
                        // Not in a game any more: back to the lobby.
                        if self.api.me().is_ok_and(|me| !me.is_in_game()) {
                            self.enter_lobby();
                        } else {
                            self.status = error;
                        }
                    }
                }
            }
        }
    }
}
