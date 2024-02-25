use rand::{seq::SliceRandom, thread_rng};

pub const MAX_CARD_NUMBER: usize = 108;

pub static ALL_CARDS: [CardTypes; MAX_CARD_NUMBER] = init_deck();

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

impl TryFrom<usize> for CardReference {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        let card_ref = CardReference::new(value);
        card_ref.ok_or(format!(
            "Card Reference {value} outside of allowed range of 0 - {MAX_CARD_NUMBER}!"
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
    pub fn is_possible_next_card(
        &self,
        next_card: &CardTypes,
        color_wish_opt: Option<Color>,
    ) -> bool {
        match (self, next_card, color_wish_opt) {
            (CardTypes::Normal(current), CardTypes::Normal(next), None) => {
                current.is_possible_next_card(next, None)
            }
            (CardTypes::Normal(_), CardTypes::Normal(next), Some(color)) => next.color == color,
            (CardTypes::Normal(_), CardTypes::Wild(_), _) => true,
            (CardTypes::Wild(_), CardTypes::Normal(_), None) => true,
            (CardTypes::Wild(_), CardTypes::Normal(next), Some(color)) => next.color == color,
            (CardTypes::Wild(_), CardTypes::Wild(_), _) => true,
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
    Regular,
    ChangeGameDirection,
    Skip,
    DrawAndColorChange(DrawAction, Color),
}

impl CardAction {
    fn is_possible_next_card(
        &self,
        current_card: &CardTypes,
        next_card: &CardTypes,
    ) -> Option<bool> {
        match self {
            CardAction::Skip => None,
            CardAction::Draw(_) => None,
            CardAction::ColorChange(color) => match next_card {
                CardTypes::Normal(next_color_card) => Some(*color == next_color_card.color),
                CardTypes::Wild(_) => Some(true),
            },
            CardAction::DrawAndColorChange(_, _) => None,
            CardAction::Regular => Some(current_card.is_possible_next_card(next_card, None)),
            CardAction::ChangeGameDirection => None,
        }
    }
}

impl Default for CardAction {
    fn default() -> Self {
        CardAction::Regular
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
            DrawAction::DrawFour => 4,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ColorCard {
    pub color: Color,
    pub symbol: ColorSymbol,
}

impl ColorCard {
    fn is_possible_next_card(&self, next_card: &ColorCard, color_wish_opt: Option<Color>) -> bool {
        match color_wish_opt {
            None => self.color == next_card.color || self.symbol == next_card.symbol,
            Some(color) => next_card.color == color,
        }
    }
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
    result.shuffle(&mut thread_rng());

    result
}

pub fn retrieve_card(card_ref: &CardReference) -> CardTypes {
    let input_card_index: usize = card_ref.into();
    let card: &_ = unsafe { ALL_CARDS.get_unchecked(input_card_index) };
    card.clone()
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

        let invalid_card_ref = CardReference::new(MAX_CARD_NUMBER + 1);
        assert!(invalid_card_ref.is_none());
    }

    #[test]
    fn card_reference_try_from_error() {
        let outside_max_range = MAX_CARD_NUMBER + 1;
        let card_ref_result: Result<CardReference, _> = outside_max_range.try_into();
        assert!(card_ref_result.is_err());
        assert_eq!(
            card_ref_result.unwrap_err(),
            "Card Reference 109 outside of allowed range of 0 - 108!"
        );
    }

    #[test]
    fn same_color_possible_next_card() {
        let color = Color::Green;
        let first_card = CardTypes::Normal(ColorCard {
            color,
            symbol: ColorSymbol::Zero,
        });
        let second_card = CardTypes::Normal(ColorCard {
            color,
            symbol: ColorSymbol::One,
        });

        assert!(first_card.is_possible_next_card(&second_card, None));
    }

    #[test]
    fn different_color_different_symbol_impossible_next_card() {
        let first_card = CardTypes::Normal(ColorCard {
            color: Color::Red,
            symbol: ColorSymbol::Zero,
        });
        let second_card = CardTypes::Normal(ColorCard {
            color: Color::Blue,
            symbol: ColorSymbol::One,
        });

        assert!(!first_card.is_possible_next_card(&second_card, None));
    }

    #[test]
    fn different_color_same_symbol_possible_next_card() {
        let symbol = ColorSymbol::Zero;
        let first_card = CardTypes::Normal(ColorCard {
            color: Color::Red,
            symbol,
        });
        let second_card = CardTypes::Normal(ColorCard {
            color: Color::Blue,
            symbol,
        });

        assert!(first_card.is_possible_next_card(&second_card, None));
    }
}
