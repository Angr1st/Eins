use eins_lib::api::{GameSetupState, JoinRequest};
use eins_lib::cards::{CardTypes, Color, ColorSymbol, WildSymbol};
use eins_lib::game::{GameDirection, GameView};
use yew::prelude::*;

use crate::{Action, App, Msg, Prompt, Screen};

fn color_name(color: Color) -> &'static str {
    match color {
        Color::Red => "Red",
        Color::Blue => "Blue",
        Color::Orange => "Orange",
        Color::Green => "Green",
    }
}

fn card_text(card: CardTypes) -> (&'static str, &'static str) {
    match card {
        CardTypes::Wild(card) => (
            "Wild",
            match card.symbol {
                WildSymbol::ChooseColor => "✦",
                WildSymbol::DrawFour => "+4",
            },
        ),
        CardTypes::Normal(card) => (
            color_name(card.color),
            match card.symbol {
                ColorSymbol::Zero => "0",
                ColorSymbol::One => "1",
                ColorSymbol::Two => "2",
                ColorSymbol::Three => "3",
                ColorSymbol::Four => "4",
                ColorSymbol::Five => "5",
                ColorSymbol::Six => "6",
                ColorSymbol::Seven => "7",
                ColorSymbol::Eight => "8",
                ColorSymbol::Nine => "9",
                ColorSymbol::DrawTwo => "+2",
                ColorSymbol::Reverse => "Reverse",
                ColorSymbol::Skip => "Skip",
            },
        ),
    }
}

fn card_face(card: CardTypes) -> Html {
    let (color, symbol) = card_text(card);
    html! { <><span class="card-corner">{color}</span><strong class={classes!((symbol.chars().count() > 2).then_some("word-symbol"))}>{symbol}</strong><span class="card-bottom">{symbol}</span></> }
}

impl App {
    pub fn render(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <div class="shell">
                <header class="masthead">
                    <a class="wordmark" href="/" aria-label="Eins home">{"eins"}<span>{"."}</span></a>
                    <span class="edition">{"THE CARD TABLE / WEB EDITION"}</span>
                    if let Some(me) = &self.me {
                        <div class="identity"><span class={classes!("dot", self.connected.then_some("online"))}></span>{me.get_nick()}</div>
                    }
                </header>
                <main>
                    if self.api.code.is_some() && !self.connected {
                        <p class="notice" role="status">{"Connecting to the table… Changes will sync when the connection returns."}</p>
                    }
                    if let Some(error) = &self.error {
                        <div class="error" role="alert"><span>{error}</span>
                            if self.api.code.is_some() {
                                <button class="quiet" disabled={self.busy} onclick={link.callback(|_| Msg::Refresh)}>{"Retry refresh"}</button>
                            }
                        </div>
                    }
                    if self.api.code.is_none() {
                        <section class="welcome">
                            <div><p class="eyebrow">{"GOOD COMPANY. ONE LAST CARD."}</p>
                                <h1>{"A seat at"}<br/>{"the table."}</h1>
                                <p class="intro">{"Match a color. Make your move. And don’t forget to call Eins."}</p>
                                <form class="register" onsubmit={link.callback(|event: SubmitEvent| { event.prevent_default(); Msg::Register })}>
                                    <label for="nickname">{"What should we call you?"}</label>
                                    <div class="input-row"><input id="nickname" ref={self.nick.clone()} placeholder="Your nickname" autocomplete="nickname" required=true maxlength="40"/>
                                    <button class="primary" disabled={self.busy} type="submit">{if self.busy { "Joining…" } else { "Find a table →" }}</button></div>
                                    <small>{"No account needed. Your seat stays saved in this tab."}</small>
                                </form>
                            </div>
                            <div class="welcome-art" aria-hidden="true"><div class="art-card art-red">{"e"}</div><div class="art-card art-orange">{"i"}</div><div class="art-card art-green">{"1"}</div></div>
                        </section>
                    } else if let Some(screen) = &self.screen {
                        {match screen {
                            Screen::Lobby(setups) => html! {
                                <section>
                                    <div class="section-heading"><div><p class="eyebrow">{"MAKE YOURSELF AT HOME"}</p><h1>{"The lobby"}</h1></div><span class="pill">{format!("{} tables", setups.len())}</span></div>
                                    <div class="lobby-grid"><div class="panel">
                                        <h2>{"Take a seat"}</h2><p class="muted">{"Join a table below. The host starts when everyone is ready."}</p>
                                        if setups.is_empty() { <div class="empty"><span>{"↗"}</span><h3>{"The first table is yours."}</h3><p>{"Create a game and invite someone to join."}</p></div> }
                                        <div class="tables">{for setups.iter().map(|setup| {
                                            let id = setup.id;
                                            let closed = setup.state == GameSetupState::Closed;
                                            html! { <div class="table-row" key={id.to_string()}><div><h3>{format!("{}’s table", setup.host_nick)}</h3><span class="muted">{format!("{} players · {}", setup.number_of_players, if closed { "Code required" } else if setup.state == GameSetupState::Full { "Full" } else { "Open to everyone" })}</span></div>
                                                <button disabled={self.busy || setup.state == GameSetupState::Full} onclick={link.callback(move |_| if closed { Msg::Prompt(Some(Prompt::Join(id))) } else { Msg::Act(Action::Join(JoinRequest { game_id: id, game_code: None })) })}>{"Join table →"}</button></div> }
                                        })}</div>
                                    </div><aside class="panel create-panel"><p class="eyebrow">{"YOUR TABLE, YOUR COMPANY"}</p><h2>{"Start something."}</h2><p>{"An open invitation, or just your friends. There’s room for everyone."}</p>
                                        <button class="primary" disabled={self.busy} onclick={link.callback(|_| Msg::Act(Action::Create(None)))}>{"Create open table"}</button>
                                        <button disabled={self.busy} onclick={link.callback(|_| Msg::Prompt(Some(Prompt::Create)))}>{"Create private table"}</button>
                                    </aside></div>
                                    {self.render_prompt(ctx)}
                                </section>
                            },
                            Screen::Waiting(setup) => {
                                let host = self.me.as_ref().is_some_and(|me| me.has_created_game_setup());
                                html! { <section class="waiting panel"><p class="eyebrow">{"A LITTLE COMPANY GOES A LONG WAY"}</p><h1>{"At the table"}</h1>
                                    <p class="muted">{if host { "Your table is in the lobby. Invite your friends to take a seat." } else { "You’re in. The host will start the game when everyone is here." }}</p>
                                    <ul class="seats">{for setup.players.iter().enumerate().map(|(i, player)| html! {
                                        <li key={player.id.to_string()}><span class="avatar">{i + 1}</span><strong>{&player.nick}</strong><span class="muted">{if player.id == setup.host_id { "Host" } else { "Ready" }}</span></li>
                                    })}</ul>
                                    <div class="actions">if host { <button class="primary" disabled={self.busy || !setup.is_ready} onclick={link.callback(|_| Msg::Act(Action::Start))}>{"Start game →"}</button> }
                                        <button disabled={self.busy} onclick={link.callback(|_| Msg::Act(Action::LeaveSetup))}>{if host { "Close table" } else { "Leave table" }}</button>
                                    </div>
                                    if !setup.is_ready { <p class="muted">{"One more player and we’re ready to play."}</p> }
                                </section> }
                            },
                            Screen::Game(view) => self.render_game(ctx, view),
                        }}
                    } else {
                        <p role="status">{"Finding your seat…"}</p>
                    }
                </main>
                <footer><span>{"A simple game. A good time."}</span><span>{"Built with Rust · Powered by eins_lib"}</span></footer>
            </div>
        }
    }

    fn render_prompt(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let cancel = link.callback(|_| Msg::Prompt(None));
        let escape = link.batch_callback(|event: KeyboardEvent| {
            (event.key() == "Escape").then_some(Msg::Prompt(None))
        });
        match self.prompt {
            Some(Prompt::Create | Prompt::Join(_)) => html! {
                <form class="panel code-form" onkeydown={escape} onsubmit={link.callback(|event: SubmitEvent| { event.prevent_default(); Msg::SubmitCode })}>
                    <h2>{if matches!(self.prompt, Some(Prompt::Create)) { "A table for your friends" } else { "You’re on the guest list" }}</h2>
                    <label for="game-code">{"Game code"}</label>
                    <div class="input-row"><input id="game-code" ref={self.code.clone()} type="password" required=true autocomplete="off"/>
                        <button class="primary" disabled={self.busy}>{if matches!(self.prompt, Some(Prompt::Create)) { "Create table" } else { "Join with code" }}</button>
                        <button type="button" onclick={cancel}>{"Cancel"}</button>
                    </div>
                </form>
            },
            Some(Prompt::Color(_)) => html! {
                <fieldset class="color-picker" onkeydown={escape}><legend>{"Choose the next color"}</legend>
                    <div class="actions" ref={self.prompt_focus.clone()} tabindex="-1">{for [Color::Red, Color::Blue, Color::Orange, Color::Green].into_iter().map(|color| html! {
                        <button class={classes!("color-choice", color_name(color))} disabled={self.busy} onclick={link.callback(move |_| Msg::PickColor(color))}>{color_name(color)}</button>
                    })}<button onclick={cancel}>{"Cancel"}</button></div>
                </fieldset>
            },
            None => Html::default(),
        }
    }

    fn render_game(&self, ctx: &Context<Self>, view: &GameView) -> Html {
        let link = ctx.link();
        let my_id = self.me.as_ref().unwrap().get_id();
        let my_turn = self.my_turn();
        let current = view.turn.as_ref().map(|turn| turn.current_player);
        let nick = |id| {
            view.players
                .iter()
                .find(|p| p.id == id)
                .map_or("Player", |p| p.nick.as_str())
        };
        let title = if let Some(winner) = view.winner {
            if winner == my_id {
                "You win!".into()
            } else {
                format!("{} wins!", nick(winner))
            }
        } else if my_turn {
            "Your move.".into()
        } else {
            format!("{}’s move.", current.map(nick).unwrap_or("Next player"))
        };
        html! {
            <section>
                <div class="section-heading"><div><p class="eyebrow">{if view.winner.is_some() { "THAT’S A WRAP" } else { "LET’S PLAY" }}</p><h1 aria-live="polite">{title}</h1></div>
                    <button class="quiet" disabled={self.busy} onclick={link.callback(|_| Msg::Act(Action::LeaveGame))}>{if view.winner.is_some() { "Back to lobby →" } else { "Leave game" }}</button>
                </div>
                <div class="players">{for view.players.iter().map(|player| html! {
                    <div class={classes!("player", (Some(player.id) == current).then_some("current"))} key={player.id.to_string()}>
                        <strong>{&player.nick}{if player.id == my_id { " (you)" } else { "" }}</strong>
                        <span>{format!("{} cards{}", player.card_count, if player.card_count == 1 { " · EINS!" } else { "" })}</span>
                    </div>
                })}</div>
                <div class="felt">
                    <div class="table-caption"><span>{"ON THE TABLE"}</span><span>{match view.direction { GameDirection::Clockwise => "↻ Clockwise", GameDirection::CounterClockwise => "↺ Counterclockwise" }}</span></div>
                    <div class="piles"><div class="deck" aria-hidden="true">{"eins."}</div><div class={classes!("card", "discard", card_text(view.top_card).0)} aria-label={format!("Top card: {} {}", card_text(view.top_card).0, card_text(view.top_card).1)}>{card_face(view.top_card)}</div>
                        <div class="turn-info">if let Some(turn) = &view.turn {
                            <span>{"COLOR IN PLAY"}</span><strong class="active-color">{color_name(turn.color)}</strong>
                            if turn.pending_draw > 0 { <p class="pending">{format!("+{} pending", turn.pending_draw)}</p><p>{"Stack a draw card or draw the penalty."}</p> }
                        } else { <strong>{"Well played."}</strong><p>{"Another round? Head back to the lobby."}</p> }</div>
                    </div>
                </div>
                <div class="hand-heading"><div><h2>{"Your hand"}<span class="count">{view.hand.len()}</span></h2><p class="muted">{if view.winner.is_some() { "The round is over. These are your remaining cards." } else if my_turn { "Choose a card to play." } else { "Take a moment. Plan your next move." }}</p></div>
                    if view.winner.is_none() { <div class="actions"><button class={classes!(self.eins.then_some("called"))} aria-pressed={self.eins.to_string()} disabled={self.busy} onclick={link.callback(|_| Msg::ToggleEins)}>{if self.eins { "Eins! ✓" } else { "Call Eins!" }}</button>
                        <button class="primary" disabled={self.busy || !my_turn} onclick={link.callback(|_| Msg::Act(Action::Draw))}>{view.turn.as_ref().filter(|t| t.pending_draw > 0).map_or("Draw a card".into(), |t| format!("Draw {} cards", t.pending_draw))}</button></div> }
                </div>
                {self.render_prompt(ctx)}
                <div class="hand">{for view.hand.iter().map(|hand_card| {
                    let id = hand_card.id;
                    let (color, symbol) = card_text(hand_card.card);
                    html! { <button key={id.card_number()} class={classes!("card", color, (self.rejected_card == Some(id)).then_some("rejected"))} disabled={self.busy || !my_turn} aria-label={format!("Play {color} {symbol}")} onclick={link.callback(move |_| Msg::SelectCard(id))} onanimationend={link.callback(move |_| Msg::ClearRejectedCard(id))}>{card_face(hand_card.card)}</button> }
                })}</div>
                if view.winner.is_none() { <p class="hand-note">{"Down to your last two? Call Eins before playing your next card."}</p> }
            </section>
        }
    }
}
