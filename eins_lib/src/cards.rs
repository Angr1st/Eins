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

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum Color {
    Red,
    Blue,
    Orange,
    Green,
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum WildSymbol {
    ChooseColor,
    DrawFour,
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct ColorCard {
    pub color: Color,
    pub symbol: ColorSymbol,
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct WildCard {
    pub symbol: WildSymbol,
}

/// Builds the 108 cards. The order is fixed, because a `CardReference` is an index into it:
/// 4 wild cards, 4 wild draw-four cards, then for each color one zero and two each of one to
/// nine, draw two, skip and reverse.
pub const fn init_deck() -> [CardTypes; MAX_CARD_NUMBER] {
    const COLORS: [Color; 4] = [Color::Red, Color::Blue, Color::Orange, Color::Green];
    const SYMBOLS: [ColorSymbol; 13] = [
        ColorSymbol::Zero,
        ColorSymbol::One,
        ColorSymbol::Two,
        ColorSymbol::Three,
        ColorSymbol::Four,
        ColorSymbol::Five,
        ColorSymbol::Six,
        ColorSymbol::Seven,
        ColorSymbol::Eight,
        ColorSymbol::Nine,
        ColorSymbol::DrawTwo,
        ColorSymbol::Skip,
        ColorSymbol::Reverse,
    ];
    let mut cards = [CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    }); MAX_CARD_NUMBER];

    // `for` loops are not allowed in a `const fn`.
    let mut index = 0;
    while index < 8 {
        let symbol = if index < 4 {
            WildSymbol::ChooseColor
        } else {
            WildSymbol::DrawFour
        };
        cards[index] = CardTypes::Wild(WildCard { symbol });
        index += 1;
    }
    let mut color = 0;
    while color < COLORS.len() {
        // Each color has its symbols twice, except for the zero, which the second round skips.
        let mut round = 0;
        while round < 2 {
            let mut symbol = round;
            while symbol < SYMBOLS.len() {
                cards[index] = CardTypes::Normal(ColorCard {
                    color: COLORS[color],
                    symbol: SYMBOLS[symbol],
                });
                index += 1;
                symbol += 1;
            }
            round += 1;
        }
        color += 1;
    }
    assert!(index == MAX_CARD_NUMBER);

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
        assert_eq!(deck.first(), Some(&first_card));
        assert_eq!(deck.last(), Some(&last_card));

        let count = |card: CardTypes| deck.iter().filter(|c| **c == card).count();
        for symbol in [WildSymbol::ChooseColor, WildSymbol::DrawFour] {
            assert_eq!(count(CardTypes::Wild(WildCard { symbol })), 4);
        }
        use ColorSymbol::*;
        for color in [Color::Red, Color::Blue, Color::Orange, Color::Green] {
            let card = |symbol| CardTypes::Normal(ColorCard { color, symbol });
            assert_eq!(count(card(Zero)), 1);
            let symbols = [
                One, Two, Three, Four, Five, Six, Seven, Eight, Nine, DrawTwo, Reverse, Skip,
            ];
            for symbol in symbols {
                assert_eq!(count(card(symbol)), 2, "{color:?} {symbol:?}");
            }
        }
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
