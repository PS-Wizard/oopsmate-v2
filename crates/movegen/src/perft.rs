use oopsmate_core::Position;

use crate::{MoveList, generate_all};

pub struct PerftCase {
    pub name: &'static str,
    pub fen: &'static str,
    pub counts: &'static [u64],
}

impl PerftCase {
    #[inline(always)]
    #[must_use]
    pub const fn nodes_at_depth(&self, depth: usize) -> Option<u64> {
        if depth < self.counts.len() {
            Some(self.counts[depth])
        } else {
            None
        }
    }

    #[must_use]
    pub fn position(&self) -> Position {
        Position::from_fen(self.fen).expect("invalid perft FEN")
    }
}

pub const STARTPOS_COUNTS: &[u64] = &[
    1,
    20,
    400,
    8_902,
    197_281,
    4_865_609,
    119_060_324,
    3_195_901_860,
    84_998_978_956,
    2_439_530_234_167,
];

pub const KIWIPETE_COUNTS: &[u64] = &[1, 48, 2_039, 97_862, 4_085_603, 193_690_690, 8_031_647_685];

pub const POSITION_3_COUNTS: &[u64] = &[
    1,
    14,
    191,
    2_812,
    43_238,
    674_624,
    11_030_083,
    178_633_661,
    3_009_794_393,
];

pub const POSITION_4_COUNTS: &[u64] = &[1, 6, 264, 9_467, 422_333, 15_833_292, 706_045_033];

pub const POSITION_5_COUNTS: &[u64] = &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194];

pub const POSITION_6_COUNTS: &[u64] = &[
    1,
    46,
    2_079,
    89_890,
    3_894_594,
    164_075_551,
    6_923_051_137,
    287_188_994_746,
    11_923_589_843_526,
    490_154_852_788_714,
];

pub const STARTPOS: PerftCase = PerftCase {
    name: "startpos",
    fen: oopsmate_core::STARTPOS_FEN,
    counts: STARTPOS_COUNTS,
};

pub const KIWIPETE: PerftCase = PerftCase {
    name: "kiwipete",
    fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    counts: KIWIPETE_COUNTS,
};

pub const POSITION_3: PerftCase = PerftCase {
    name: "position3",
    fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    counts: POSITION_3_COUNTS,
};

pub const POSITION_4: PerftCase = PerftCase {
    name: "position4",
    fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    counts: POSITION_4_COUNTS,
};

pub const POSITION_5: PerftCase = PerftCase {
    name: "position5",
    fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    counts: POSITION_5_COUNTS,
};

pub const POSITION_6: PerftCase = PerftCase {
    name: "position6",
    fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    counts: POSITION_6_COUNTS,
};

pub const PERFT_CASES: &[PerftCase] = &[
    STARTPOS, KIWIPETE, POSITION_3, POSITION_4, POSITION_5, POSITION_6,
];

#[must_use]
pub fn perft(position: &mut Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut moves = MoveList::new();
    generate_all(position, &mut moves);
    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0u64;
    for &mv in moves.as_slice() {
        position.make_move(mv);
        nodes += perft(position, depth - 1);
        position.unmake_move(mv);
    }

    nodes
}
