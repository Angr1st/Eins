use std::time::Duration;

use eins_lib::api::*;
use eins_lib::game::GameView;
use eins_lib::infrastructure::Player;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use gloo_timers::future::TimeoutFuture;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use yew::Callback;

#[derive(Clone)]
pub struct Api {
    base: String,
    http: Client,
    pub code: Option<String>,
}

pub struct Error {
    pub message: String,
    pub status: Option<u16>,
    pub expired: bool,
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self {
            message: format!("Could not reach the server: {error}"),
            status: None,
            expired: false,
        }
    }
}

pub enum Screen {
    Lobby(Vec<GameSetupResponse>),
    Waiting(GameSetupStateResponse),
    Game(GameView),
}

pub struct Snapshot {
    pub me: Player,
    pub screen: Screen,
}

pub enum Notice {
    Connected,
    Disconnected,
    Changed(ServerEvent),
    Expired,
}

#[derive(Clone)]
pub enum Action {
    Create(Option<String>),
    Join(JoinRequest),
    Start,
    LeaveSetup,
    Play(PlayRequest),
    Draw,
    LeaveGame,
}

impl Api {
    pub fn new(code: Option<String>) -> Self {
        let origin = web_sys::window().unwrap().location().origin().unwrap();
        Self {
            base: format!("{origin}/api"),
            http: Client::new(),
            code,
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let request = self.http.request(method, format!("{}{path}", self.base));
        match &self.code {
            Some(code) => request.header("code", code),
            None => request,
        }
    }

    async fn send(request: RequestBuilder) -> Result<Response, Error> {
        let response = request.send().await?;
        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(Error {
                message: if message.trim().is_empty() {
                    status.to_string()
                } else {
                    message
                },
                status: Some(status.as_u16()),
                expired: status.as_u16() == 401,
            })
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        Ok(Self::send(
            self.request(Method::GET, path)
                .timeout(Duration::from_secs(10)),
        )
        .await?
        .json()
        .await?)
    }

    async fn post(&self, path: &str, body: &impl Serialize) -> Result<Response, Error> {
        Self::send(
            self.request(Method::POST, path)
                .timeout(Duration::from_secs(10))
                .json(body),
        )
        .await
    }

    pub async fn register(&self, nick: String) -> Result<Player, Error> {
        let response: RegistrationResponse = self
            .post("/player/register", &Registration { nick })
            .await?
            .json()
            .await?;
        Ok(response.player)
    }

    pub async fn snapshot(&self) -> Result<Snapshot, Error> {
        let response: MeResponse = self.get("/player/me").await.map_err(|mut error| {
            error.expired |= error.status == Some(404);
            error
        })?;
        let me = response.player;
        let screen = if me.is_in_game() {
            Screen::Game(self.get("/game/me").await?)
        } else if me.is_free() {
            let setups: GameSetupsResponse = self.get("/game/setup").await?;
            Screen::Lobby(setups.setups)
        } else {
            Screen::Waiting(self.get("/game/setup/me").await?)
        };
        Ok(Snapshot { me, screen })
    }

    pub async fn act(&self, action: Action) -> Result<(), Error> {
        match action {
            Action::Create(game_code) => {
                self.post("/game/setup", &Setup { game_code }).await?;
            }
            Action::Join(request) => {
                let response: JoinResponse = self
                    .post("/game/setup/join", &request)
                    .await?
                    .json()
                    .await?;
                if response.state == JoinResponseState::Failure {
                    return Err(Error {
                        message: "Wrong game code, or the table is full.".into(),
                        status: None,
                        expired: false,
                    });
                }
            }
            Action::Play(request) => {
                self.post("/game/play", &request).await?;
            }
            other => {
                let path = match other {
                    Action::Start => "/game/setup/start",
                    Action::LeaveSetup => "/game/setup/leave",
                    Action::Draw => "/game/draw",
                    Action::LeaveGame => "/game/leave",
                    _ => unreachable!(),
                };
                self.post(path, &()).await?;
            }
        }
        Ok(())
    }

    /// Fetch-based SSE can send the secret header; native EventSource cannot.
    /// Never put the player's credential in an event URL or query string.
    pub async fn listen(&self, notice: Callback<Notice>) {
        loop {
            match Self::send(self.request(Method::GET, "/events")).await {
                Ok(response) => {
                    notice.emit(Notice::Connected);
                    let mut stream = response.bytes_stream().eventsource();
                    while let Some(Ok(event)) = stream.next().await {
                        if let Ok(event) = serde_json::from_str(&event.data) {
                            notice.emit(Notice::Changed(event));
                        }
                    }
                }
                Err(error) if error.expired || error.status == Some(404) => {
                    notice.emit(Notice::Expired);
                    return;
                }
                Err(_) => {}
            }
            notice.emit(Notice::Disconnected);
            TimeoutFuture::new(1_000).await;
        }
    }
}
