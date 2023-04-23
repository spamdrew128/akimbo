use super::{consts::*, position::{Position, Move, ratt, batt}};

macro_rules! bitloop {($bb:expr, $sq:ident, $func:expr) => {
    while $bb > 0 {
        let $sq = $bb.trailing_zeros() as u8;
        $bb &= $bb - 1;
        $func;
    }
}}

pub struct List<T> {
    pub list: [T; 252],
    pub len: usize,
}

pub type MoveList = List<Move>;
pub type ScoreList = List<i16>;

impl<T> List<T> {
    pub fn uninit() -> Self {
        #[allow(clippy::uninit_assumed_init, invalid_value)]
        Self { list: unsafe { std::mem::MaybeUninit::uninit().assume_init() }, len: 0 }
    }
}

impl MoveList {
    #[inline]
    pub fn push(&mut self, from: u8, to: u8, flag: u8, mpc: usize) {
        self.list[self.len] = Move { from, to, flag, mpc: mpc as u8 };
        self.len += 1;
    }

    pub fn pick(&mut self, scores: &mut ScoreList) -> Option<(Move, i16)> {
        if self.len == 0 { return None }
        let (mut idx, mut best) = (0, i16::MIN);
        for i in 0..self.len {
            let score = scores.list[i];
            if score > best {
                best = score;
                idx = i;
            }
        }
        self.len -= 1;
        scores.list.swap(idx, self.len);
        self.list.swap(idx, self.len);
        Some((self.list[self.len], best))
    }
}

#[inline]
fn encode<const PC: usize, const FLAG: u8>(moves: &mut MoveList, mut attacks: u64, from: u8) {
    bitloop!(attacks, to, moves.push(from, to, FLAG, PC))
}

impl Position {
    pub fn gen<const QUIETS: bool>(&self) -> MoveList {
        let mut moves = MoveList::uninit();
        let (side, occ) = (usize::from(self.c), self.bb[0] | self.bb[1]);
        let (boys, opps) = (self.bb[side], self.bb[side ^ 1]);
        let pawns = self.bb[P] & boys;
        if QUIETS {
            if self.cr & CS[side] > 0 && !self.in_check() {self.castles(&mut moves, occ)}
            if side == WH {pushes::<WH>(&mut moves, !occ, pawns)} else {pushes::<BL>(&mut moves, !occ, pawns)}
        }
        if self.enp > 0 {en_passants(&mut moves, pawns, self.enp, side)}
        pawn_caps(&mut moves, pawns, opps, side);
        pc_moves::<N, QUIETS>(&mut moves, occ, opps, boys & self.bb[N]);
        pc_moves::<B, QUIETS>(&mut moves, occ, opps, boys & self.bb[B]);
        pc_moves::<R, QUIETS>(&mut moves, occ, opps, boys & self.bb[R]);
        pc_moves::<Q, QUIETS>(&mut moves, occ, opps, boys & self.bb[Q]);
        pc_moves::<K, QUIETS>(&mut moves, occ, opps, boys & self.bb[K]);
        moves
    }

    fn castles(&self, moves: &mut MoveList, occ: u64) {
        if self.c {
            if self.cr & BQS > 0 && occ & BD8 == 0 && !self.is_sq_att(59, BL, occ) {moves.push(60, 58, QS, K)}
            if self.cr & BKS > 0 && occ & FG8 == 0 && !self.is_sq_att(61, BL, occ) {moves.push(60, 62, KS, K)}
        } else {
            if self.cr & WQS > 0 && occ & BD1 == 0 && !self.is_sq_att(3, WH, occ) {moves.push(4, 2, QS, K)}
            if self.cr & WKS > 0 && occ & FG1 == 0 && !self.is_sq_att(5, WH, occ) {moves.push(4, 6, KS, K)}
        }
    }
}

fn pc_moves<const PC: usize, const QUIETS: bool>(moves: &mut MoveList, occ: u64, opps: u64, mut attackers: u64) {
    bitloop!(attackers, from, {
        let attacks = match PC {
            N => NATT[from as usize],
            R => ratt(from as usize, occ),
            B => batt(from as usize, occ),
            Q => ratt(from as usize, occ) | batt(from as usize, occ),
            K => KATT[from as usize],
            _ => 0
        };
        encode::<PC, CAP>(moves, attacks & opps, from);
        if QUIETS { encode::<PC, QUIET>(moves, attacks & !occ, from) }
    });
}

fn pawn_caps(moves: &mut MoveList, pawns: u64, opps: u64, c: usize) {
    let (mut attackers, mut promo) = (pawns & !PENRANK[c], pawns & PENRANK[c]);
    bitloop!(attackers, from, encode::<P, CAP>(moves, PATT[c][from as usize] & opps, from));
    bitloop!(promo, from, {
        let mut attacks = PATT[c][from as usize] & opps;
        bitloop!(attacks, to, for flag in NPC..=QPC { moves.push(from, to, flag, P) })
    });
}

fn en_passants(moves: &mut MoveList, pawns: u64, sq: u8, c: usize) {
    let mut attackers = PATT[c ^ 1][sq as usize] & pawns;
    bitloop!(attackers, from, moves.push(from, sq, ENP, P))
}

fn shift<const SIDE: usize>(bb: u64) -> u64 {
    if SIDE == WH {bb >> 8} else {bb << 8}
}

fn idx_shift<const SIDE: usize, const AMOUNT: u8>(idx: u8) -> u8 {
    if SIDE == WH {idx + AMOUNT} else {idx - AMOUNT}
}

fn pushes<const SIDE: usize>(moves: &mut MoveList, empty: u64, pawns: u64) {
    let mut dbl = shift::<SIDE>(shift::<SIDE>(empty & DBLRANK[SIDE]) & empty) & pawns;
    let mut push = shift::<SIDE>(empty) & pawns;
    let mut promo = push & PENRANK[SIDE];
    push &= !PENRANK[SIDE];
    bitloop!(push, from, moves.push(from, idx_shift::<SIDE, 8>(from), QUIET, P));
    bitloop!(promo, from, for flag in NPR..=QPR {moves.push(from, idx_shift::<SIDE, 8>(from), flag, P)});
    bitloop!(dbl, from, moves.push(from, idx_shift::<SIDE, 16>(from), DBL, P));
}
