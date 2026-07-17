use dioxus_free_icons::icons::ld_icons::{
    LdBraces, LdBrackets, LdCircleDot, LdCircleSlash, LdDot, LdHash, LdOption, LdParentheses,
    LdQuote, LdSquare, LdSquareCheckBig, LdType,
};

// pub(crate) type Bool = LdToggleLeft;
pub(crate) type BoolTrue = LdSquareCheckBig;
pub(crate) type BoolFalse = LdSquare;
pub(crate) type Char = LdType;
pub(crate) type Map = LdBraces;
pub(crate) type MapKey = LdDot;
pub(crate) type MapValue = LdCircleDot;
pub(crate) type MapEmpty = LdBraces;
pub(crate) type Number = LdHash;
pub(crate) type Option = LdOption;
pub(crate) type OptionNone = LdCircleSlash;
pub(crate) type String = LdQuote;
// pub(crate) type Bytes = LdBinary;
pub(crate) type Seq = LdBrackets;
// pub(crate) type SeqEmpty = LdBrackets;
pub(crate) type Unit = LdParentheses;
