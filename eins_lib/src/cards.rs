use rand::{rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};

pub const MAX_CARD_NUMBER: usize = 108;

pub static ALL_CARDS: [CardTypes; MAX_CARD_NUMBER] = init_deck();

pub fn get_card(card_ref: &CardReference) -> CardTypes {
    let index = card_ref.card_number();
    ALL_CARDS[index]
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "usize")]
pub struct CardReference(usize);

impl CardReference {
    fn new(card_number: usize) -> Option<CardReference> {
        if card_number >= MAX_CARD_NUMBER {
            None
        } else {
            Some(CardReference(card_number))
        }
    }

    pub fn card_number(&self) -> usize {
        self.0
    }
}

impl From<&CardReference> for usize {
    fn from(value: &CardReference) -> Self {
        value.0
    }
}

impl TryFrom<usize> for CardReference {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let card_ref = CardReference::new(value);
        card_ref.ok_or(format!(
            "Card Reference {value} outside of allowed range of 0 - {}!",
            MAX_CARD_NUMBER - 1
        ))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Color {
    Red,
    Blue,
    Orange,
    Green,
}

#[derive(Debug, PartialEq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum WildSymbol {
    ChooseColor,
    DrawFour,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CardTypes {
    Normal(ColorCard),
    Wild(WildCard),
}

impl CardTypes {
    /// Whether this card may be played on `top` while `color` is the color in play:
    /// the top card's color, or the color chosen with a wild card.
    pub fn can_be_played_on(&self, top: &CardTypes, color: Color) -> bool {
        match (self, top) {
            (CardTypes::Wild(_), _) => true,
            (CardTypes::Normal(card), CardTypes::Normal(top)) => {
                card.color == color || card.symbol == top.symbol
            }
            (CardTypes::Normal(card), CardTypes::Wild(_)) => card.color == color,
        }
    }

    pub fn is_possible_initial_card(&self) -> bool {
        match self {
            CardTypes::Normal(_) => true,
            CardTypes::Wild(_) => false,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ColorCard {
    pub color: Color,
    pub symbol: ColorSymbol,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct WildCard {
    pub symbol: WildSymbol,
}

pub const fn init_deck() -> [CardTypes; MAX_CARD_NUMBER] {
    let mut cards: [CardTypes; MAX_CARD_NUMBER] = [CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Zero,
    }); MAX_CARD_NUMBER];
    let mut index: usize = 0;
    //4 Times Choose Color
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });
    index = index + 1;
    //4 Times Draw Four
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::DrawFour,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::DrawFour,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::DrawFour,
    });
    index = index + 1;
    cards[index] = CardTypes::Wild(WildCard {
        symbol: WildSymbol::DrawFour,
    });
    index = index + 1;
    //Red Normal Cards
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Zero,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Red,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    //Blue Normal Cards
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Zero,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Blue,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    //Orange Normal Cards
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Zero,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Orange,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    //Green Normal Cards
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Zero,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Reverse,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::One,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Two,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Three,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Four,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Five,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Six,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Seven,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Eight,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Nine,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::DrawTwo,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Skip,
    });
    index = index + 1;
    cards[index] = CardTypes::Normal(ColorCard {
        color: Color::Green,
        symbol: ColorSymbol::Reverse,
    });

    cards
}

pub fn create_deck() -> Vec<CardReference> {
    let mut result = Vec::with_capacity(MAX_CARD_NUMBER);
    for i in 0..MAX_CARD_NUMBER {
        result.push(CardReference::new(i).expect("CardReference should always work!"))
    }
    result.shuffle(&mut rng());

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
    fn check_deck() {
        let deck = init_deck();
        let first_card = CardTypes::Wild(WildCard {
            symbol: WildSymbol::ChooseColor,
        });
        let last_card = CardTypes::Normal(ColorCard {
            color: Color::Green,
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

        assert!(CardReference::new(MAX_CARD_NUMBER - 1).is_some());
        assert!(CardReference::new(MAX_CARD_NUMBER).is_none());
    }

    #[test]
    fn card_reference_try_from_error() {
        let card_ref_result: Result<CardReference, _> = MAX_CARD_NUMBER.try_into();
        assert_eq!(
            card_ref_result.unwrap_err(),
            "Card Reference 108 outside of allowed range of 0 - 107!"
        );
    }

    #[test]
    fn card_reference_deserialize_is_validated() {
        let last: CardReference = serde_json::from_str("107").unwrap();
        assert_eq!(last.card_number(), 107);
        assert!(serde_json::from_str::<CardReference>("108").is_err());
        assert!(serde_json::from_str::<CardReference>("5000").is_err());
    }

    #[test]
    fn same_color_possible_next_card() {
        let top = CardTypes::Normal(ColorCard {
            color: Color::Green,
            symbol: ColorSymbol::Zero,
        });
        let card = CardTypes::Normal(ColorCard {
            color: Color::Green,
            symbol: ColorSymbol::One,
        });

        assert!(card.can_be_played_on(&top, Color::Green));
    }

    #[test]
    fn different_color_different_symbol_impossible_next_card() {
        let top = CardTypes::Normal(ColorCard {
            color: Color::Red,
            symbol: ColorSymbol::Zero,
        });
        let card = CardTypes::Normal(ColorCard {
            color: Color::Blue,
            symbol: ColorSymbol::One,
        });

        assert!(!card.can_be_played_on(&top, Color::Red));
    }

    #[test]
    fn different_color_same_symbol_possible_next_card() {
        let top = CardTypes::Normal(ColorCard {
            color: Color::Red,
            symbol: ColorSymbol::Skip,
        });
        let card = CardTypes::Normal(ColorCard {
            color: Color::Blue,
            symbol: ColorSymbol::Skip,
        });

        assert!(card.can_be_played_on(&top, Color::Red));
    }

    #[test]
    fn only_the_chosen_color_follows_a_wild_card() {
        let top = CardTypes::Wild(WildCard {
            symbol: WildSymbol::ChooseColor,
        });
        let green = CardTypes::Normal(ColorCard {
            color: Color::Green,
            symbol: ColorSymbol::Two,
        });
        let red = CardTypes::Normal(ColorCard {
            color: Color::Red,
            symbol: ColorSymbol::Two,
        });

        assert!(green.can_be_played_on(&top, Color::Green));
        assert!(!red.can_be_played_on(&top, Color::Green));
        assert!(top.can_be_played_on(&top, Color::Green));
    }
}
