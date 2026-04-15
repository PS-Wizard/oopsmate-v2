use crate::types::{Piece, Square};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MoveKind {
    #[default]
    Quiet = 0,
    DoublePush = 1,
    Castle = 2,
    EnPassant = 3,
    Capture = 4,
    PromotionKnight = 8,
    PromotionBishop = 9,
    PromotionRook = 10,
    PromotionQueen = 11,
    CapturePromotionKnight = 12,
    CapturePromotionBishop = 13,
    CapturePromotionRook = 14,
    CapturePromotionQueen = 15,
}

impl MoveKind {
    #[inline(always)]
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Quiet,
            1 => Self::DoublePush,
            2 => Self::Castle,
            3 => Self::EnPassant,
            4 => Self::Capture,
            8 => Self::PromotionKnight,
            9 => Self::PromotionBishop,
            10 => Self::PromotionRook,
            11 => Self::PromotionQueen,
            12 => Self::CapturePromotionKnight,
            13 => Self::CapturePromotionBishop,
            14 => Self::CapturePromotionRook,
            15 => Self::CapturePromotionQueen,
            _ => panic!("invalid move kind"),
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_capture(self) -> bool {
        (self as u8 & 0x4) != 0
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_promotion(self) -> bool {
        (self as u8 & 0x8) != 0
    }

    #[inline(always)]
    #[must_use]
    pub const fn promotion_piece(self) -> Option<Piece> {
        match self {
            Self::PromotionKnight | Self::CapturePromotionKnight => Some(Piece::Knight),
            Self::PromotionBishop | Self::CapturePromotionBishop => Some(Piece::Bishop),
            Self::PromotionRook | Self::CapturePromotionRook => Some(Piece::Rook),
            Self::PromotionQueen | Self::CapturePromotionQueen => Some(Piece::Queen),
            _ => None,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Move(pub u16);

impl Move {
    pub const NULL: Self = Self(0);

    #[inline(always)]
    #[must_use]
    pub const fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        Self((from.raw() as u16) | ((to.raw() as u16) << 6) | ((kind as u16) << 12))
    }

    #[inline(always)]
    #[must_use]
    pub const fn from(self) -> Square {
        Square::from_raw((self.0 & 0x3f) as u8)
    }

    #[inline(always)]
    #[must_use]
    pub const fn to(self) -> Square {
        Square::from_raw(((self.0 >> 6) & 0x3f) as u8)
    }

    #[inline(always)]
    #[must_use]
    pub const fn kind(self) -> MoveKind {
        MoveKind::from_u8((self.0 >> 12) as u8)
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_capture(self) -> bool {
        ((self.0 >> 12) & 0x4) != 0
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_promotion(self) -> bool {
        ((self.0 >> 12) & 0x8) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_packs_and_unpacks() {
        let mv = Move::new(
            Square::from_algebraic("e2").unwrap(),
            Square::from_algebraic("e4").unwrap(),
            MoveKind::DoublePush,
        );

        assert_eq!(mv.from(), Square::from_algebraic("e2").unwrap());
        assert_eq!(mv.to(), Square::from_algebraic("e4").unwrap());
        assert_eq!(mv.kind(), MoveKind::DoublePush);
        assert!(!mv.is_capture());
        assert!(!mv.is_promotion());
    }
}
