use eins_lib::api::GameSetupState;
use eins_lib::cards::{CardTypes, Color as CardColor, ColorSymbol, WildSymbol};
use eins_lib::game::{GameDirection, GameView};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use ratatui::Frame;
use uuid::Uuid;

use crate::app::{Action, App, PromptKind, Screen};

const CARD_WIDTH: u16 = 7;
const CARD_HEIGHT: u16 = 5;
const COLORS: [CardColor; 4] = [
    CardColor::Red,
    CardColor::Blue,
    CardColor::Orange,
    CardColor::Green,
];

type Clicks = Vec<(Rect, Action)>;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let mut clicks = Clicks::new();
    let [main, status, help] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let help_text = match &app.screen {
        Screen::Register { .. } => {
            draw_register(frame, main, app);
            "Enter register · Esc quit"
        }
        Screen::Lobby { setups, selected } => {
            draw_lobby(frame, main, app, setups, *selected, &mut clicks);
            "↑↓ select · Enter join · n new game · p new private game · q quit"
        }
        Screen::Waiting { setup } => {
            draw_waiting(frame, main, app, setup.as_ref(), &mut clicks);
            "s start (creator) · l leave · q quit"
        }
        Screen::Game {
            view,
            selected,
            eins,
            picking_color,
        } => {
            if let Some(view) = view {
                draw_game(frame, main, app, view, *selected, *eins, &mut clicks);
            }
            if picking_color.is_some() {
                draw_color_picker(frame, main, &mut clicks);
                "1-4 or click a color · Esc cancel"
            } else {
                "←→ select · Enter play · d draw · e Eins · L leave game · q quit"
            }
        }
    };

    if let Some(prompt) = &app.prompt {
        let title = match prompt.kind {
            PromptKind::PrivateGameCode => " Game code for your private game ",
            PromptKind::JoinGameCode(_) => " This game needs a code ",
        };
        let area = main.centered(Constraint::Length(50), Constraint::Length(3));
        frame.render_widget(Clear, area);
        let text = Paragraph::new(format!("{}▏", prompt.text)).block(
            Block::bordered()
                .title(title)
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(text, area);
    }

    frame.render_widget(Paragraph::new(app.status.as_str()).yellow(), status);
    frame.render_widget(Paragraph::new(help_text).dark_gray(), help);
    app.clickables = clicks;
}

fn draw_register(frame: &mut Frame, area: Rect, app: &App) {
    let Screen::Register { nick } = &app.screen else {
        return;
    };
    let area = area.centered(Constraint::Length(40), Constraint::Length(5));
    let lines = vec![
        Line::from("Welcome to Eins!").bold().centered(),
        Line::from(""),
        Line::from(vec![
            Span::raw("Nick: "),
            Span::raw(format!("{nick}▏")).bold(),
        ]),
    ];
    let block = Block::bordered()
        .title(" Register ")
        .border_type(BorderType::Rounded);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn button(frame: &mut Frame, area: Rect, label: &str, style: Style) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(style);
    frame.render_widget(
        Paragraph::new(label).style(style).centered().block(block),
        area,
    );
}

/// Lays out buttons left to right and makes them clickable.
fn buttons(frame: &mut Frame, area: Rect, items: &[(&str, Action, Style)], clicks: &mut Clicks) {
    let mut x = area.x;
    for (label, action, style) in items {
        let width = label.chars().count() as u16 + 4;
        let rect = Rect::new(x, area.y, width, 3).intersection(area);
        button(frame, rect, label, *style);
        clicks.push((rect, *action));
        x += width + 1;
    }
}

fn draw_lobby(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    setups: &[eins_lib::api::GameSetupResponse],
    selected: usize,
    clicks: &mut Clicks,
) {
    let me = app.me.as_ref().map(|p| p.get_nick()).unwrap_or_default();
    let block = Block::bordered()
        .title(format!(" Eins · {me} · Lobby "))
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [list, actions] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(inner);

    if setups.is_empty() {
        frame.render_widget(
            Paragraph::new("No games yet. Create one with n.").dark_gray(),
            list,
        );
    }
    for (i, setup) in setups.iter().enumerate().take(list.height as usize) {
        let row = Rect::new(list.x, list.y + i as u16, list.width, 1);
        let (state, state_style) = match setup.state {
            GameSetupState::Open => ("open", Style::new().green()),
            GameSetupState::Closed => ("needs code", Style::new().yellow()),
            GameSetupState::Full => ("full", Style::new().red()),
        };
        let line = Line::from(vec![
            Span::raw(format!(" {:<16}", setup.host_nick)),
            Span::raw(format!(
                "{:>2} player{}   ",
                setup.number_of_players,
                if setup.number_of_players == 1 {
                    ""
                } else {
                    "s"
                }
            )),
            Span::styled(state, state_style),
        ]);
        let style = if i == selected {
            Style::new().reversed()
        } else {
            Style::new()
        };
        frame.render_widget(Paragraph::new(line).style(style), row);
        clicks.push((row, Action::JoinSetup(i)));
    }

    buttons(
        frame,
        actions,
        &[
            ("New game (n)", Action::CreateOpen, Style::new()),
            ("New private game (p)", Action::CreatePrivate, Style::new()),
        ],
        clicks,
    );
}

fn draw_waiting(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    setup: Option<&eins_lib::api::GameSetupStateResponse>,
    clicks: &mut Clicks,
) {
    let creator = app.me.as_ref().is_some_and(|p| p.has_created_game_setup());
    let block = Block::bordered()
        .title(" Eins · Waiting room ")
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [list, actions] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(inner);

    let mut lines = vec![];
    if let Some(setup) = setup {
        lines.push(Line::from(format!("Players ({}):", setup.players.len())).bold());
        for player in &setup.players {
            let host = if player.id == setup.id { " (host)" } else { "" };
            lines.push(Line::from(format!("  • {}{host}", player.nick)));
        }
        lines.push(Line::from(""));
        lines.push(if !setup.is_ready {
            Line::from("Waiting for more players…").dark_gray()
        } else if creator {
            Line::from("Ready! Press s to start.").green()
        } else {
            Line::from("Waiting for the host to start…").green()
        });
    }
    frame.render_widget(Paragraph::new(lines), list);

    let mut items = vec![];
    if creator {
        items.push(("Start (s)", Action::Start, Style::new().green()));
    }
    items.push(("Leave (l)", Action::LeaveSetup, Style::new()));
    buttons(frame, actions, &items, clicks);
}

fn color_of(color: CardColor) -> Color {
    match color {
        CardColor::Red => Color::Rgb(200, 45, 45),
        CardColor::Blue => Color::Rgb(40, 95, 205),
        CardColor::Orange => Color::Rgb(230, 140, 20),
        CardColor::Green => Color::Rgb(40, 155, 70),
    }
}

fn color_name(color: CardColor) -> &'static str {
    match color {
        CardColor::Red => "Red",
        CardColor::Blue => "Blue",
        CardColor::Orange => "Orange",
        CardColor::Green => "Green",
    }
}

fn card_face(card: &CardTypes) -> (String, Color) {
    match card {
        CardTypes::Normal(card) => {
            let label = match card.symbol {
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
                ColorSymbol::Reverse => "⇄",
                ColorSymbol::Skip => "⊘",
            };
            (label.to_string(), color_of(card.color))
        }
        CardTypes::Wild(card) => {
            let label = match card.symbol {
                WildSymbol::ChooseColor => "★",
                WildSymbol::DrawFour => "+4",
            };
            (label.to_string(), Color::Rgb(35, 35, 35))
        }
    }
}

/// Blends a color towards gray, for cards that cannot be played right now.
fn faded(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let mix = |c: u8| ((c as u16 + 2 * 70) / 3) as u8;
            Color::Rgb(mix(r), mix(g), mix(b))
        }
        other => other,
    }
}

fn draw_card(frame: &mut Frame, area: Rect, card: &CardTypes, selected: bool, playable: bool) {
    let (label, mut background) = card_face(card);
    if !playable {
        background = faded(background);
    }
    let (border, border_style) = if selected {
        (BorderType::Thick, Style::new().fg(Color::Yellow).bold())
    } else {
        (BorderType::Rounded, Style::new().fg(Color::White))
    };
    let block = Block::bordered()
        .border_type(border)
        .border_style(border_style)
        .style(Style::new().bg(background));
    // Wild cards get a band in all four colors above and below their symbol.
    let band = |glyph: &'static str| -> Line {
        match card {
            CardTypes::Wild(_) => Line::from(
                COLORS
                    .iter()
                    .map(|c| Span::styled(glyph, Style::new().fg(color_of(*c))))
                    .collect::<Vec<_>>(),
            )
            .centered(),
            CardTypes::Normal(_) => Line::from(""),
        }
    };
    let face = Paragraph::new(vec![band("▄"), Line::from(label).centered(), band("▀")])
        .style(Style::new().fg(Color::White).bold())
        .block(block);
    frame.render_widget(face, area);
}

fn draw_game(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    view: &GameView,
    selected: usize,
    eins: bool,
    clicks: &mut Clicks,
) {
    let me = app.my_id();
    let my_nick = app.me.as_ref().map(|p| p.get_nick()).unwrap_or_default();
    // Nicks come with the game view; everyone named in it is a player of this game.
    let nick = |id: &Uuid| {
        view.players
            .iter()
            .find(|p| p.id == *id)
            .map_or("?", |p| p.nick.as_str())
    };
    let block = Block::bordered()
        .title(format!(" Eins · {my_nick} "))
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let per_row = (inner.width.saturating_sub(2) / CARD_WIDTH).max(1);
    let hand_rows = (view.hand.len() as u16).div_ceil(per_row).max(1);
    let [players_area, table_area, hand_area, actions_area, _] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(CARD_HEIGHT + 1),
        Constraint::Length(hand_rows * CARD_HEIGHT + 2),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .areas(inner);

    // Players in seating order with their card counts.
    let current = view.turn.as_ref().map(|t| t.current_player);
    let mut spans = vec![Span::raw(match view.direction {
        GameDirection::Clockwise => "↻  ",
        GameDirection::CounterClockwise => "↺  ",
    })];
    for player in &view.players {
        let is_current = Some(player.id) == current;
        let mut name = player.nick.clone();
        if Some(player.id) == me {
            name.push_str(" (you)");
        }
        let style = if is_current {
            Style::new().yellow().bold()
        } else {
            Style::new()
        };
        spans.push(Span::styled(
            format!(
                "{}{name}: {}",
                if is_current { "▶ " } else { "" },
                player.card_count
            ),
            style,
        ));
        if player.card_count == 1 {
            spans.push(Span::styled(" EINS!", Style::new().red().bold()));
        }
        spans.push(Span::raw("    "));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), players_area);

    // The discard pile and what happens next.
    let top = Rect::new(table_area.x + 1, table_area.y + 1, CARD_WIDTH, CARD_HEIGHT)
        .intersection(table_area);
    draw_card(frame, top, &view.top_card, false, true);
    let info_area = Rect::new(
        top.right() + 3,
        top.y,
        table_area.right().saturating_sub(top.right() + 3),
        CARD_HEIGHT,
    );
    let my_turn = current.is_some() && current == me;
    let mut info = vec![];
    if let Some(turn) = &view.turn {
        info.push(Line::from(vec![
            Span::raw("Color: "),
            Span::styled("   ", Style::new().bg(color_of(turn.color))),
            Span::raw(format!(" {}", color_name(turn.color))),
        ]));
        if turn.pending_draw > 0 {
            info.push(
                Line::from(format!(
                    "+{} pending: stack a +2 or +4, or draw them",
                    turn.pending_draw
                ))
                .red()
                .bold(),
            );
        }
        info.push(Line::from(""));
        info.push(if my_turn {
            Line::from("Your turn!").green().bold()
        } else {
            Line::from(format!("Waiting for {}…", nick(&turn.current_player))).dark_gray()
        });
    }
    if let Some(winner) = view.winner {
        let text = if Some(winner) == me {
            "You win!".to_string()
        } else {
            format!("{} wins!", nick(&winner))
        };
        info.push(Line::from(text).green().bold());
    }
    frame.render_widget(Paragraph::new(info), info_area);

    // The own hand; cards that cannot be played on this turn are faded.
    let playable = view.turn.as_ref().map(|t| &t.playable);
    let hand_block = Block::bordered()
        .title(format!(" Your hand ({}) ", view.hand.len()))
        .border_type(BorderType::Rounded);
    let hand_inner = hand_block.inner(hand_area);
    frame.render_widget(hand_block, hand_area);
    for (i, card) in view.hand.iter().enumerate() {
        let i16 = i as u16;
        let rect = Rect::new(
            hand_inner.x + (i16 % per_row) * CARD_WIDTH,
            hand_inner.y + (i16 / per_row) * CARD_HEIGHT,
            CARD_WIDTH,
            CARD_HEIGHT,
        );
        if rect.bottom() > hand_inner.bottom() {
            break;
        }
        let can_play = !my_turn || playable.is_some_and(|p| p.contains(&card.id));
        draw_card(frame, rect, &card.card, i == selected && my_turn, can_play);
        clicks.push((rect, Action::PlayCard(i)));
    }

    // Buttons.
    let items: Vec<(&str, Action, Style)> = if view.winner.is_some() {
        vec![("Back to lobby (L)", Action::LeaveGame, Style::new().green())]
    } else {
        let eins_style = if eins {
            Style::new().black().on_yellow().bold()
        } else {
            Style::new().yellow()
        };
        vec![
            ("Draw (d)", Action::Draw, Style::new()),
            (
                if eins { "EINS! ✓ (e)" } else { "Eins! (e)" },
                Action::ToggleEins,
                eins_style,
            ),
            (
                "Leave (L)",
                Action::LeaveGame,
                Style::new().add_modifier(Modifier::DIM),
            ),
        ]
    };
    buttons(frame, actions_area, &items, clicks);
}

fn draw_color_picker(frame: &mut Frame, area: Rect, clicks: &mut Clicks) {
    let popup = area.centered(Constraint::Length(52), Constraint::Length(7));
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .title(" Choose a color ")
        .border_type(BorderType::Rounded);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    for (i, color) in COLORS.iter().enumerate() {
        let rect = Rect::new(inner.x + 1 + i as u16 * 12, inner.y + 1, 11, 3).intersection(inner);
        let style = Style::new().fg(Color::White).bg(color_of(*color)).bold();
        button(
            frame,
            rect,
            &format!("{} {}", i + 1, color_name(*color)),
            style,
        );
        clicks.push((rect, Action::PickColor(*color)));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use crossterm::event::{KeyCode, KeyEvent};
    use eins_lib::api::{GameSetupResponse, GameSetupStatePlayerResponse, GameSetupStateResponse};
    use eins_lib::cards::{CardReference, ColorCard, WildCard, ALL_CARDS, MAX_CARD_NUMBER};
    use eins_lib::game::{HandCard, PlayerView, TurnView};
    use eins_lib::infrastructure::Player;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    use super::*;
    use crate::api::Api;

    fn app(nick: &str) -> App {
        // Nothing listens on port 9. These tests only use actions that stay offline.
        let mut app = App::new(Api::new("http://127.0.0.1:9".into()), mpsc::channel().0);
        app.me = Some(Player::new(nick.into()));
        app
    }

    fn render(app: &mut App) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(100, 34)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// The screen as text, one line per terminal row.
    fn lines(buffer: &Buffer) -> Vec<String> {
        let area = buffer.area;
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer[(x, y)].symbol().chars().next().unwrap_or(' '))
                    .collect()
            })
            .collect()
    }

    /// Where `text` is drawn on screen.
    fn find(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
        let needle: Vec<char> = text.chars().collect();
        lines(buffer).iter().enumerate().find_map(|(y, line)| {
            let line: Vec<char> = line.chars().collect();
            let x = line.windows(needle.len()).position(|w| w == needle)?;
            Some((x as u16, y as u16))
        })
    }

    fn shows(buffer: &Buffer, text: &str) -> bool {
        find(buffer, text).is_some()
    }

    fn click(app: &mut App, text: &str) {
        let buffer = render(app);
        let (x, y) = find(&buffer, text).unwrap_or_else(|| panic!("{text:?} is not on screen"));
        app.on_click(x, y);
    }

    fn c(color: CardColor, symbol: ColorSymbol) -> CardTypes {
        CardTypes::Normal(ColorCard { color, symbol })
    }

    const WILD: CardTypes = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });

    /// A hand of the given cards, each with its own card id.
    fn hand(cards: &[CardTypes]) -> Vec<HandCard> {
        let mut used = [false; MAX_CARD_NUMBER];
        cards
            .iter()
            .map(|card| {
                let i = (0..MAX_CARD_NUMBER)
                    .find(|&i| !used[i] && ALL_CARDS[i] == *card)
                    .unwrap();
                used[i] = true;
                HandCard {
                    id: CardReference::try_from(i).unwrap(),
                    card: *card,
                }
            })
            .collect()
    }

    /// Alice (the app's player) against bob, who holds 5 cards. `playable` indexes `cards`.
    fn game(app: &App, cards: &[CardTypes], playable: &[usize], current: Option<&str>) -> GameView {
        let alice = app.my_id().unwrap();
        let bob = Uuid::new_v4();
        let hand = hand(cards);
        GameView {
            game_id: alice,
            state: "Play".into(),
            players: vec![
                PlayerView {
                    id: alice,
                    nick: "alice".into(),
                    card_count: hand.len(),
                },
                PlayerView {
                    id: bob,
                    nick: "bob".into(),
                    card_count: 5,
                },
            ],
            turn: current.map(|nick| TurnView {
                current_player: if nick == "alice" { alice } else { bob },
                color: CardColor::Red,
                pending_draw: 0,
                playable: playable.iter().map(|&i| hand[i].id).collect(),
            }),
            winner: None,
            hand,
            top_card: c(CardColor::Red, ColorSymbol::Five),
            direction: GameDirection::Clockwise,
        }
    }

    fn show_game(app: &mut App, view: GameView) {
        app.screen = Screen::Game {
            view: Some(view),
            selected: 0,
            eins: false,
            picking_color: None,
        };
    }

    #[test]
    fn the_table_shows_turn_counts_and_pending_stack() {
        let mut app = app("alice");
        let cards = [
            c(CardColor::Red, ColorSymbol::One),
            WILD,
            c(CardColor::Blue, ColorSymbol::Two),
        ];
        let mut view = game(&app, &cards, &[0, 1], Some("alice"));
        view.turn.as_mut().unwrap().pending_draw = 4;
        show_game(&mut app, view);
        let screen = render(&mut app);

        assert!(shows(&screen, "▶ alice (you): 3"));
        assert!(shows(&screen, "bob: 5"));
        assert!(shows(&screen, "Color:"));
        assert!(shows(&screen, "+4 pending"));
        assert!(shows(&screen, "Your turn!"));
        assert!(shows(&screen, "Your hand (3)"));
    }

    #[test]
    fn other_players_are_named_with_their_nicks() {
        let mut app = app("alice");
        let cards = [c(CardColor::Red, ColorSymbol::One)];
        let view = game(&app, &cards, &[], Some("bob"));
        show_game(&mut app, view);
        let screen = render(&mut app);
        assert!(shows(&screen, "▶ bob: 5"));
        assert!(shows(&screen, "Waiting for bob…"));

        let mut view = game(&app, &cards, &[], None);
        view.winner = Some(view.players[1].id);
        show_game(&mut app, view);
        assert!(shows(&render(&mut app), "bob wins!"));
    }

    #[test]
    fn unplayable_cards_are_faded_on_your_turn_only() {
        let red_one = c(CardColor::Red, ColorSymbol::One);
        let red_two = c(CardColor::Red, ColorSymbol::Two);
        let backgrounds = |current: &str| {
            let mut app = app("alice");
            let view = game(&app, &[red_one, red_two], &[0], Some(current));
            show_game(&mut app, view);
            let screen = render(&mut app);
            // Both cards are red; only their playability differs.
            let card_background = |index: usize| {
                let (area, _) = app
                    .clickables
                    .iter()
                    .find(|(_, action)| matches!(action, Action::PlayCard(i) if *i == index))
                    .unwrap();
                screen[(area.x + area.width / 2, area.y + area.height / 2)].bg
            };
            (card_background(0), card_background(1))
        };

        let (playable, unplayable) = backgrounds("alice");
        assert_ne!(playable, unplayable);
        let (first, second) = backgrounds("bob");
        assert_eq!((first, second), (playable, playable));
    }

    #[test]
    fn clicking_eins_and_a_wild_card() {
        let mut app = app("alice");
        let cards = [c(CardColor::Red, ColorSymbol::One), WILD];
        let view = game(&app, &cards, &[0, 1], Some("alice"));
        let wild_id = view.hand[1].id;
        show_game(&mut app, view);

        click(&mut app, "Eins! (e)");
        assert!(matches!(app.screen, Screen::Game { eins: true, .. }));
        assert!(shows(&render(&mut app), "EINS! ✓"));

        click(&mut app, "★");
        assert!(
            matches!(app.screen, Screen::Game { picking_color: Some(id), .. } if id == wild_id)
        );
        let screen = render(&mut app);
        assert!(shows(&screen, "Choose a color"));
        assert!(shows(&screen, "4 Green"));

        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert!(!shows(&render(&mut app), "Choose a color"));
    }

    #[test]
    fn the_lobby_lists_games_by_host() {
        let mut app = app("dave");
        let setup = |host_nick: &str, number_of_players, state| GameSetupResponse {
            id: Uuid::new_v4(),
            host_nick: host_nick.into(),
            number_of_players,
            state,
        };
        app.screen = Screen::Lobby {
            setups: vec![
                setup("alice", 2, GameSetupState::Open),
                setup("carol", 1, GameSetupState::Closed),
            ],
            selected: 0,
        };
        let screen = render(&mut app);
        let lines = lines(&screen);
        let row = |nick: &str| lines.iter().find(|l| l.contains(nick)).unwrap().clone();
        assert!(row("alice").contains(" 2 players   open"));
        assert!(row("carol").contains(" 1 player   needs code"));

        // Joining a game that needs a code asks for it first.
        click(&mut app, "carol");
        assert!(shows(&render(&mut app), "This game needs a code"));
    }

    #[test]
    fn only_the_host_can_start() {
        let mut host = app("alice");
        let host_id = host.my_id().unwrap();
        let setup = || GameSetupStateResponse {
            id: host_id,
            players: vec![
                GameSetupStatePlayerResponse {
                    id: host_id,
                    nick: "alice".into(),
                },
                GameSetupStatePlayerResponse {
                    id: Uuid::new_v4(),
                    nick: "bob".into(),
                },
            ],
            is_ready: true,
        };

        host.me.as_mut().unwrap().create_game_setup();
        host.screen = Screen::Waiting {
            setup: Some(setup()),
        };
        let screen = render(&mut host);
        assert!(shows(&screen, "alice (host)"));
        assert!(shows(&screen, "Start (s)"));

        let mut guest = app("bob");
        guest.me.as_mut().unwrap().join_game_setup(host_id);
        guest.screen = Screen::Waiting {
            setup: Some(setup()),
        };
        let screen = render(&mut guest);
        assert!(!shows(&screen, "Start (s)"));
        assert!(shows(&screen, "Waiting for the host to start…"));
    }
}
