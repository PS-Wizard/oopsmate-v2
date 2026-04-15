use std::mem::MaybeUninit;

use oopsmate_core::Move;

pub const MAX_MOVES: usize = 256;

#[derive(Clone, Debug)]
pub struct MoveList {
    moves: [MaybeUninit<Move>; MAX_MOVES],
    len: usize,
}

impl MoveList {
    #[inline(always)]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            moves: [MaybeUninit::uninit(); MAX_MOVES],
            len: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    pub fn push(&mut self, mv: Move) {
        debug_assert!((self.len as usize) < MAX_MOVES);
        unsafe {
            self.moves.get_unchecked_mut(self.len).write(mv);
        }
        self.len += 1;
    }

    #[inline(always)]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    #[must_use]
    pub fn as_slice(&self) -> &[Move] {
        unsafe { std::slice::from_raw_parts(self.moves.as_ptr() as *const Move, self.len) }
    }

    #[inline(always)]
    #[must_use]
    pub fn contains(&self, mv: Move) -> bool {
        self.as_slice().contains(&mv)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::{MoveKind, Square};

    #[test]
    fn list_push_clear_and_contains_work() {
        let mut list = MoveList::new();
        let mv = Move::new(
            Square::from_algebraic("e2").unwrap(),
            Square::from_algebraic("e4").unwrap(),
            MoveKind::DoublePush,
        );

        list.push(mv);

        assert_eq!(list.len(), 1);
        assert!(list.contains(mv));

        list.clear();
        assert!(list.is_empty());
    }
}
