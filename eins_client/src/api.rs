use std::io::{BufRead, BufReader};
use std::sync::mpsc::Sender;
use std::time::Duration;

use eins_lib::api::{
    GameSetupResponse, GameSetupStateResponse, GameSetupsResponse, JoinRequest, JoinResponse,
    MeResponse, PlayRequest, PlayerResponse, PlayersResponse, Registration, RegistrationResponse,
    ServerEvent, Setup,
};
use eins_lib::cards::{CardReference, Color};
use eins_lib::game::GameView;
use eins_lib::infrastructure::Player;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::Method;
use serde::de::DeserializeOwned;
use uuid::Uuid;

/// What the event stream tells the UI thread.
pub enum Notice {
    Connected,
    Disconnected,
    Event(ServerEvent),
}

/// Blocking client for the Eins server. Errors are the server's message, ready to show.
pub struct Api {
    base: String,
    http: Client,
    code: Option<String>,
}

impl Api {
    pub fn new(base: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("the HTTP client can be built");
        Self {
            base,
            http,
            code: None,
        }
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let request = self.http.request(method, format!("{}{path}", self.base));
        match &self.code {
            Some(code) => request.header("code", code),
            None => request,
        }
    }

    fn send(request: RequestBuilder) -> Result<reqwest::blocking::Response, String> {
        let response = request
            .send()
            .map_err(|e| format!("Server not reachable: {e}"))?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            Err(if text.is_empty() {
                status.to_string()
            } else {
                text
            })
        }
    }

    fn json<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, String> {
        Self::send(request)?
            .json()
            .map_err(|e| format!("Unexpected server response: {e}"))
    }

    pub fn register(&mut self, nick: String) -> Result<Player, String> {
        let request = self
            .request(Method::POST, "/player/register")
            .json(&Registration { nick });
        let response: RegistrationResponse = Self::json(request)?;
        self.code = Some(response.player.get_code().to_string());
        Ok(response.player)
    }

    pub fn me(&self) -> Result<Player, String> {
        let response: MeResponse = Self::json(self.request(Method::GET, "/player/me"))?;
        Ok(response.player)
    }

    pub fn players(&self) -> Result<Vec<PlayerResponse>, String> {
        let response: PlayersResponse = Self::json(self.request(Method::GET, "/player"))?;
        Ok(response.players)
    }

    pub fn setups(&self) -> Result<Vec<GameSetupResponse>, String> {
        let response: GameSetupsResponse = Self::json(self.request(Method::GET, "/game/setup"))?;
        Ok(response.setups)
    }

    pub fn create_setup(&self, game_code: Option<String>) -> Result<(), String> {
        let request = self
            .request(Method::POST, "/game/setup")
            .json(&Setup { game_code });
        Self::send(request).map(|_| ())
    }

    pub fn join(&self, game_id: Uuid, game_code: Option<String>) -> Result<JoinResponse, String> {
        let request = self
            .request(Method::POST, "/game/setup/join")
            .json(&JoinRequest { game_id, game_code });
        Self::json(request)
    }

    pub fn my_setup(&self) -> Result<GameSetupStateResponse, String> {
        Self::json(self.request(Method::GET, "/game/setup/me"))
    }

    pub fn leave_setup(&self) -> Result<(), String> {
        Self::send(self.request(Method::POST, "/game/setup/leave")).map(|_| ())
    }

    pub fn start(&self) -> Result<(), String> {
        Self::send(self.request(Method::POST, "/game/setup/start")).map(|_| ())
    }

    pub fn game(&self) -> Result<GameView, String> {
        Self::json(self.request(Method::GET, "/game/me"))
    }

    pub fn play(
        &self,
        card: CardReference,
        color: Option<Color>,
        eins: bool,
    ) -> Result<GameView, String> {
        let request =
            self.request(Method::POST, "/game/play")
                .json(&PlayRequest { card, color, eins });
        Self::json(request)
    }

    pub fn draw(&self) -> Result<GameView, String> {
        Self::json(self.request(Method::POST, "/game/draw"))
    }

    pub fn leave_game(&self) -> Result<(), String> {
        Self::send(self.request(Method::POST, "/game/leave")).map(|_| ())
    }

    /// Listens to the server's event stream on a background thread and forwards it to
    /// `notices`, reconnecting every second while the server is unreachable. Call after
    /// registering. The thread ends when the receiver is dropped.
    pub fn listen(&self, notices: Sender<Notice>) {
        let url = format!("{}/events", self.base);
        let code = self
            .code
            .clone()
            .expect("listen is called after registering");
        std::thread::spawn(move || {
            // No timeout: the stream stays open for as long as the server runs.
            let http = Client::builder()
                .timeout(None)
                .build()
                .expect("the HTTP client can be built");
            loop {
                let response = http.get(&url).header("code", &code).send();
                if let Ok(response) = response.and_then(|r| r.error_for_status()) {
                    if notices.send(Notice::Connected).is_err() {
                        return;
                    }
                    for line in BufReader::new(response).lines() {
                        let Ok(line) = line else { break };
                        let Some(data) = line.strip_prefix("data:") else {
                            continue;
                        };
                        if let Ok(event) = serde_json::from_str(data.trim()) {
                            if notices.send(Notice::Event(event)).is_err() {
                                return;
                            }
                        }
                    }
                }
                if notices.send(Notice::Disconnected).is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    }
}
