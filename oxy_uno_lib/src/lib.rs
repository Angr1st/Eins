#[macro_use]
extern crate specs_derive;

pub mod cards {
    use specs::prelude::*;

    pub const MAX_CARD_NUMBER: i32 = 108;

    #[derive(Debug)]
    pub enum Color{
        Red,
        Blue,
        Orange,
        Green
    }

    #[derive(Debug)]
    pub enum CardNumbers {
        Zero,
        One,
        Two,
        Three,
        Four,
        Five,
        Six,
        Seven,
        Eight,
        Nine
    }

    #[derive(Debug)]
    pub enum Symbol{
        Number(CardNumbers),
        Draw,
        Wild,
        Reverse,
        Skip
    }

    #[derive(Component)]
    pub struct Reversable {}

    #[derive(Component)]
    pub struct Skip {}

    #[derive(Component)]
    pub struct Wild {
        pub next_color: Color
    }

    #[derive(Component)]
    pub struct Drawable {
        pub number_of_cards_to_draw: DrawAction
    }

    #[derive(Debug)]
    pub enum DrawAction{
        DrawOne,
        DrawTwo,
        DrawFour
    }

    #[derive(Debug)]
    pub enum Category{

    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn number_is_108() {
        assert_eq!(108, crate::cards::MAX_CARD_NUMBER);
    }
}
