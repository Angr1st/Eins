use rand::{seq::SliceRandom, thread_rng};

pub const MAX_CARD_NUMBER: usize = 108;

#[derive(Debug, Copy, Clone)]
pub struct CardReference(usize);

impl CardReference {
    fn new(card_number: usize) -> Option<CardReference> {
        if card_number > MAX_CARD_NUMBER {
            None
        } else {
            Some(CardReference(card_number))
        }
    }

    fn card_number(&self) -> usize {
        self.0
    }
}

impl From<&CardReference> for usize {
    fn from(value: &CardReference) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Color {
    Red,
    Blue,
    Orange,
    Green,
}

#[derive(Debug, PartialEq)]
pub enum ColorSymbol {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    DrawTwo,
    Reverse,
    Skip,
}

#[derive(Debug, PartialEq)]
pub enum WildSymbol {
    ChooseColor,
    DrawFour,
}

#[derive(Debug, PartialEq)]
pub enum CardTypes {
    Normal(ColorCard),
    Wild(WildCard),
}

impl CardTypes {
    fn is_possible_next_card(&self, next_card: &CardTypes) -> bool {
        match (self, next_card) {
            (CardTypes::Normal(current), CardTypes::Normal(next)) => current.is_possible_next_card(next),
            (CardTypes::Normal(_), CardTypes::Wild(_)) => true,
            (CardTypes::Wild(_), CardTypes::Normal(_)) => true,
            (CardTypes::Wild(_), CardTypes::Wild(_)) => true,
        }
    }

    pub fn is_possible_initial_card(&self) -> bool {
        match self {
            CardTypes::Normal(_) => true,
            CardTypes::Wild(_) => false,
        }
    }
}

#[derive(Debug)]
pub enum CardAction {
    Draw(DrawAction),
    ColorChange(Color),
    Default,
    ChangeGameDirection,
    Skip,
}

impl CardAction {
    fn is_possible_next_card(&self, current_card: &CardTypes, next_card: &CardTypes) -> Option<bool> {
        match self {
            CardAction::Skip => None,
            CardAction::Draw(_) => None,
            CardAction::ColorChange(color) => match next_card {
                CardTypes::Normal(next_color_card) => Some(*color == next_color_card.color),
                CardTypes::Wild(_) => Some(true),
            },
            CardAction::Default => Some(current_card.is_possible_next_card(next_card)),
            CardAction::ChangeGameDirection => None,
        }
    }
}

impl Default for CardAction {
    fn default() -> Self {
        CardAction::Default
    }
}

#[derive(Debug)]
pub enum DrawAction {
    DrawOne,
    DrawTwo,
    DrawFour,
}

impl Default for DrawAction {
    fn default() -> Self {
        DrawAction::DrawOne
    }
}

impl From<&DrawAction> for u8 {
    fn from(value: &DrawAction) -> Self {
        match value {
            DrawAction::DrawOne => 1,
            DrawAction::DrawTwo => 2,
            DrawAction::DrawFour => 4
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ColorCard {
    pub color: Color,
    pub symbol: ColorSymbol,
}

impl ColorCard {
    fn is_possible_next_card(&self, next_card: &ColorCard) -> bool {
        self.color == next_card.color
    }
}

#[derive(Debug, PartialEq)]
pub struct WildCard {
    pub symbol: WildSymbol,
}

pub fn init_deck() -> Vec<CardTypes> {
    let mut cards = Vec::with_capacity(MAX_CARD_NUMBER);
    for _ in 0..4 {
        cards.push(CardTypes::Wild(WildCard {
            symbol: WildSymbol::ChooseColor,
        }));
    }
    for _ in 0..4 {
        cards.push(CardTypes::Wild(WildCard {
            symbol: WildSymbol::DrawFour,
        }));
    }
    for color in [Color::Red, Color::Blue, Color::Green, Color::Orange].iter() {
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Zero,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::One,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Two,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Three,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Four,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Five,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Six,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Seven,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Eight,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Nine,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::DrawTwo,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Skip,
        }));
        cards.push(CardTypes::Normal(ColorCard {
            color: *color,
            symbol: ColorSymbol::Reverse,
        }));
    }
    cards
}

pub fn create_deck() -> Vec<CardReference> {
    let mut result =  Vec::with_capacity(MAX_CARD_NUMBER);
    for i in 0..MAX_CARD_NUMBER {
        result.push(CardReference::new(i).expect("CardReference should always work!"))
    }
    result.shuffle(&mut thread_rng());

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number_is_108() {
        assert_eq!(108, MAX_CARD_NUMBER);
    }

    #[test]
    fn for_loop() {
        let mut current = 0;
        for _ in 0..4 {
            current = current + 1;
        }
        assert_eq!(4, current);
    }

    #[test]
    fn check_deck() {
        let deck = init_deck();
        let first_card = CardTypes::Wild(WildCard {
            symbol: WildSymbol::ChooseColor,
        });
        let last_card = CardTypes::Normal(ColorCard {
            color: Color::Orange,
            symbol: ColorSymbol::Reverse,
        });

        let first_deck_card = match deck.first() {
            Some(it) => it,
            _ => return,
        };
        assert_eq!(first_deck_card, &first_card);
        let last_deck_card = match deck.last() {
            Some(it) => it,
            _ => return,
        };
        assert_eq!(last_deck_card, &last_card);
    }

    #[test]
    fn card_reference_creation() {
        let zero = CardReference::new(0);
        match zero {
            Some(card_ref) => assert_eq!(card_ref.card_number(), 0),
            None => return,
        }

        let invalid_card_ref = CardReference::new(MAX_CARD_NUMBER + 1);
        assert!(invalid_card_ref.is_none());
    }
}
