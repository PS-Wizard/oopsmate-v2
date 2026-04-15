use crate::types::{CastlingRights, EMPTY_SQUARE, Square};

pub const MAX_POSITION_HISTORY: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Undo {
    pub moved: u8,
    pub captured: u8,
    pub castling: CastlingRights,
    pub ep_square: Square,
    pub rule50: u16,
    pub fullmove: u16,
    pub hash: u64,
}

impl Default for Undo {
    fn default() -> Self {
        Self {
            moved: EMPTY_SQUARE,
            captured: EMPTY_SQUARE,
            castling: CastlingRights::NONE,
            ep_square: Square::NONE,
            rule50: 0,
            fullmove: 1,
            hash: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UndoStack {
    entries: [Undo; MAX_POSITION_HISTORY],
    len: usize,
}

impl UndoStack {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            entries: [Undo::default(); MAX_POSITION_HISTORY],
            len: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, undo: Undo) {
        debug_assert!(self.len < MAX_POSITION_HISTORY, "undo stack overflow");
        self.entries[self.len] = undo;
        self.len += 1;
    }

    #[inline(always)]
    pub(crate) fn pop(&mut self) -> Undo {
        debug_assert!(self.len != 0, "undo stack underflow");
        self.len -= 1;
        self.entries[self.len]
    }

    #[inline(always)]
    pub(crate) fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RepetitionStack {
    hashes: [u64; MAX_POSITION_HISTORY],
    len: usize,
}

impl RepetitionStack {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            hashes: [0; MAX_POSITION_HISTORY],
            len: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, hash: u64) {
        debug_assert!(self.len < MAX_POSITION_HISTORY, "repetition stack overflow");
        self.hashes[self.len] = hash;
        self.len += 1;
    }

    #[inline(always)]
    pub(crate) fn pop(&mut self) -> u64 {
        debug_assert!(self.len != 0, "repetition stack underflow");
        self.len -= 1;
        self.hashes[self.len]
    }

    #[inline(always)]
    pub(crate) fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    #[must_use]
    pub(crate) const fn get(&self, index: usize) -> u64 {
        self.hashes[index]
    }
}

impl Default for RepetitionStack {
    fn default() -> Self {
        Self::new()
    }
}
