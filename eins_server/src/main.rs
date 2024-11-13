use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::cards;
use eins_lib::game::Play;
use std::fmt::Write;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(index));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> impl IntoResponse {
    let game = eins_lib::test().expect("Test should succeed!");
    let first_player = game
        .get_players()
        .first()
        .expect("There is always a first player");
    // format!("{}", game);
    let mut output = String::new();
    writeln!(output, "Current card: {:?}", game.get_current_card()).unwrap();
    writeln!(output, "Current hand cards:").unwrap();
    let hand = first_player.get_held_cards();
    for card_ref in hand.iter() {
        let card = cards::get_card(card_ref);
        writeln!(output, "Card: {:?}", card).unwrap();
    }
    writeln!(output, "Choices of the current player.").unwrap();
    let choices = game.get_available_choices();
    match choices {
        Play::PossibleCards { options } => {
            for card_ref in options {
                let card = cards::get_card(card_ref);
                writeln!(output, "Card: {:?}", card).unwrap();
            }
        }
        Play::DrawCards { draw_amount } => {
            writeln!(output, "you have to draw: {:?}", draw_amount).unwrap()
        }
    }
    output
}
