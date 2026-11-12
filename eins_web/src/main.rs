mod api;
mod ui;

use api::{Action, Api, Error, Notice, Screen, Snapshot};
use eins_lib::api::{JoinRequest, PlayRequest, ServerEvent};
use eins_lib::cards::{CardReference, CardTypes, Color};
use eins_lib::infrastructure::Player;
use futures_util::future::{AbortHandle, Abortable};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;
use yew::prelude::*;

const SESSION_KEY: &str = "eins.player-code";

#[wasm_bindgen(module = "/src/motion.js")]
extern "C" {
    fn capture_cards() -> JsValue;
    fn animate_cards(before: JsValue);
    fn cancel_card_motion();
}

enum Prompt {
    Create,
    Join(Uuid),
    Color(CardReference),
}

enum Msg {
    Register,
    Registered(Result<Player, Error>),
    Refresh,
    Loaded(Result<Snapshot, Error>),
    Notice(Notice),
    Act(Action),
    Acted(Result<(), Error>),
    Prompt(Option<Prompt>),
    SubmitCode,
    SelectCard(CardReference),
    ClearRejectedCard(CardReference),
    PickColor(Color),
    ToggleEins,
}

struct App {
    api: Api,
    me: Option<Player>,
    screen: Option<Screen>,
    prompt: Option<Prompt>,
    nick: NodeRef,
    code: NodeRef,
    prompt_focus: NodeRef,
    focus_prompt: bool,
    eins: bool,
    busy: bool,
    refresh_queued: bool,
    connected: bool,
    error: Option<String>,
    listener: Option<AbortHandle>,
    rejected_card: Option<CardReference>,
    card_layout: Option<JsValue>,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

impl App {
    fn listen(&mut self, ctx: &Context<Self>) {
        let (handle, registration) = AbortHandle::new_pair();
        let api = self.api.clone();
        let callback = ctx.link().callback(Msg::Notice);
        wasm_bindgen_futures::spawn_local(async move {
            let _ = Abortable::new(api.listen(callback), registration).await;
        });
        self.listener = Some(handle);
    }

    fn fail(&mut self, error: Error) {
        if error.expired {
            cancel_card_motion();
            self.card_layout = None;
            if let Some(listener) = self.listener.take() {
                listener.abort();
            }
            if let Some(storage) = storage() {
                let _ = storage.remove_item(SESSION_KEY);
            }
            self.api.code = None;
            self.me = None;
            self.screen = None;
            self.prompt = None;
            self.eins = false;
            self.connected = false;
            self.refresh_queued = false;
            self.error = Some("Your session has ended. Choose a nickname to join again.".into());
        } else {
            self.error = Some(error.message);
        }
    }

    fn refresh(&mut self, ctx: &Context<Self>) {
        if self.api.code.is_none() {
            return;
        }
        if self.busy {
            self.refresh_queued = true;
            return;
        }
        self.busy = true;
        self.refresh_queued = false;
        let api = self.api.clone();
        ctx.link()
            .send_future(async move { Msg::Loaded(api.snapshot().await) });
    }

    fn play(&self, ctx: &Context<Self>, card: CardReference, color: Option<Color>) {
        ctx.link().send_message(Msg::Act(Action::Play(PlayRequest {
            card,
            color,
            eins: self.eins,
        })));
    }

    fn my_turn(&self) -> bool {
        match (&self.screen, &self.me) {
            (Some(Screen::Game(view)), Some(me)) => view
                .turn
                .as_ref()
                .is_some_and(|turn| turn.current_player == me.get_id()),
            _ => false,
        }
    }

    fn can_play(&self, card: CardReference) -> bool {
        matches!(&self.screen, Some(Screen::Game(view)) if view.turn.as_ref()
            .is_some_and(|turn| turn.playable.contains(&card)))
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let code = storage().and_then(|s| s.get_item(SESSION_KEY).ok().flatten());
        let mut app = Self {
            api: Api::new(code),
            me: None,
            screen: None,
            prompt: None,
            nick: NodeRef::default(),
            code: NodeRef::default(),
            prompt_focus: NodeRef::default(),
            focus_prompt: false,
            eins: false,
            busy: false,
            refresh_queued: false,
            connected: false,
            error: None,
            listener: None,
            rejected_card: None,
            card_layout: None,
        };
        if app.api.code.is_some() {
            app.listen(ctx);
            app.refresh(ctx);
        }
        app
    }

    fn update(&mut self, ctx: &Context<Self>, message: Msg) -> bool {
        match message {
            Msg::Register => {
                if self.busy {
                    return false;
                }
                let nick = self.nick.cast::<HtmlInputElement>().unwrap().value();
                if nick.trim().is_empty() {
                    self.error = Some("Please enter a nickname.".into());
                    return true;
                }
                self.busy = true;
                self.error = None;
                let api = self.api.clone();
                ctx.link().send_future(async move {
                    Msg::Registered(api.register(nick.trim().into()).await)
                });
            }
            Msg::Registered(result) => {
                self.busy = false;
                match result {
                    Ok(me) => {
                        self.api.code = Some(me.get_code().into());
                        if let Some(storage) = storage() {
                            let _ = storage.set_item(SESSION_KEY, me.get_code());
                        }
                        self.me = Some(me);
                        self.listen(ctx);
                        self.refresh(ctx);
                    }
                    Err(error) => self.fail(error),
                }
            }
            Msg::Refresh => {
                self.error = None;
                self.refresh(ctx);
            }
            Msg::Loaded(result) => {
                self.busy = false;
                match result {
                    Ok(snapshot) => {
                        let changed_screen = self.screen.as_ref().is_none_or(|old| {
                            std::mem::discriminant(old) != std::mem::discriminant(&snapshot.screen)
                        });
                        // Capture the old DOM only for updates within this round. Initial
                        // deals and restored sessions should appear immediately.
                        self.card_layout = match (&self.screen, &snapshot.screen) {
                            (Some(Screen::Game(old)), Screen::Game(new))
                                if old.game_id == new.game_id =>
                            {
                                Some(capture_cards())
                            }
                            _ => {
                                cancel_card_motion();
                                None
                            }
                        };
                        self.me = Some(snapshot.me);
                        self.screen = Some(snapshot.screen);
                        if changed_screen {
                            self.prompt = None;
                            self.eins = false;
                            self.rejected_card = None;
                        }
                        if let Some(Prompt::Color(card)) = self.prompt {
                            if !self.can_play(card) {
                                self.prompt = None;
                            }
                        }
                    }
                    Err(error) => self.fail(error),
                }
                if self.refresh_queued {
                    self.refresh(ctx);
                }
            }
            Msg::Notice(notice) => match notice {
                Notice::Connected => {
                    self.connected = true;
                    self.refresh(ctx);
                }
                Notice::Disconnected => self.connected = false,
                Notice::Expired => self.refresh(ctx),
                Notice::Changed(event) => {
                    let relevant = match (event, &self.screen) {
                        (ServerEvent::Resync, _) | (_, None) => true,
                        (ServerEvent::Lobby, Some(Screen::Lobby(_) | Screen::Waiting(_))) => true,
                        (ServerEvent::Game { game_id }, Some(Screen::Game(view))) => {
                            game_id == view.game_id
                        }
                        _ => false,
                    };
                    if relevant {
                        self.refresh(ctx);
                    }
                }
            },
            Msg::Act(action) => {
                if self.busy {
                    return false;
                }
                self.busy = true;
                self.error = None;
                let api = self.api.clone();
                ctx.link()
                    .send_future(async move { Msg::Acted(api.act(action).await) });
            }
            Msg::Acted(result) => {
                self.busy = false;
                match result {
                    Ok(()) => {
                        self.prompt = None;
                        self.eins = false;
                    }
                    Err(error) => self.fail(error),
                }
                // Read again even after a rejected move: someone else may have moved.
                self.refresh(ctx);
            }
            Msg::Prompt(prompt) => {
                self.prompt = prompt;
                self.focus_prompt = self.prompt.is_some();
            }
            Msg::SubmitCode => {
                let code = self.code.cast::<HtmlInputElement>().unwrap().value();
                if code.trim().is_empty() {
                    self.error = Some("Enter a game code, or cancel to use an open table.".into());
                    return true;
                }
                let action = match self.prompt {
                    Some(Prompt::Create) => Action::Create(Some(code)),
                    Some(Prompt::Join(game_id)) => Action::Join(JoinRequest {
                        game_id,
                        game_code: Some(code),
                    }),
                    _ => return false,
                };
                ctx.link().send_message(Msg::Act(action));
            }
            Msg::SelectCard(id) => {
                if self.busy || !self.my_turn() {
                    return false;
                }
                if !self.can_play(id) {
                    self.rejected_card = Some(id);
                    return true;
                }
                self.rejected_card = None;
                if matches!(eins_lib::cards::get_card(&id), CardTypes::Wild(_)) {
                    self.prompt = Some(Prompt::Color(id));
                    self.focus_prompt = true;
                } else {
                    self.play(ctx, id, None);
                }
            }
            Msg::ClearRejectedCard(id) => {
                if self.rejected_card == Some(id) {
                    self.rejected_card = None;
                }
            }
            Msg::PickColor(color) => {
                if let Some(Prompt::Color(card)) = self.prompt {
                    if !self.busy && self.can_play(card) {
                        self.play(ctx, card, Some(color));
                    }
                }
            }
            Msg::ToggleEins => self.eins = !self.eins,
        }
        true
    }

    fn rendered(&mut self, _: &Context<Self>, _: bool) {
        if let Some(before) = self.card_layout.take() {
            animate_cards(before);
        }
        if self.focus_prompt {
            let target = match self.prompt {
                Some(Prompt::Create | Prompt::Join(_)) => &self.code,
                _ => &self.prompt_focus,
            };
            if let Some(element) = target.cast::<web_sys::HtmlElement>() {
                let _ = element.focus();
            }
            self.focus_prompt = false;
        }
    }

    fn destroy(&mut self, _: &Context<Self>) {
        cancel_card_motion();
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        self.render(ctx)
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
