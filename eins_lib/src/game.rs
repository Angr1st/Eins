use card::CardReference;

pub struct game_session {
    stack:Vec<CardReference>,
    deck:Vec<CardReference>,
    players:Vec<hand>
}

pub struct hand {
    player_id:Guid,
    held_cards:Vec<CardReference>
}

