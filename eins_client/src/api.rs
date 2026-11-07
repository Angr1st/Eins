use std::time::Duration;

use eins_lib::api::{
    GameSetupResponse, GameSetupStateResponse, GameSetupsResponse, JoinRequest, JoinResponse,
    MeResponse, PlayRequest, PlayerResponse, PlayersResponse, Registration, RegistrationResponse,
    Setup,
};
use eins_lib::cards::{CardReference, Color};
use eins_lib::game::GameView;
use eins_lib::infrastructure::Player;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::Method;
use serde::de::DeserializeOwned;
use uuid::Uuid;

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
}
