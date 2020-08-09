use anyhow::{anyhow, Result};

use crate::{WIDTH, HEIGHT};

#[derive(Copy, Clone)]
struct StaticMasks {
    bottom_mask: u64,
    full_board_mask: u64,
}
impl StaticMasks {
    pub fn new() -> Self {
        Self {
            bottom_mask: Self::bottom(),
            full_board_mask: Self::full_board(),
        }
    }
    fn bottom() -> u64 {
        let mut mask = 0;
        for column in 0..(WIDTH as u64) {
            mask |= 1 << (column * (HEIGHT as u64 + 1))
        }
        mask
    }
    fn full_board() -> u64 {
        Self::bottom() * ((1 << HEIGHT as u64) - 1)
    }
}
#[derive(Copy, Clone)]
pub struct BitBoard {
    pub player_mask: u64,
    pub board_mask: u64,
    static_masks: StaticMasks,
    num_moves: usize,
}
impl BitBoard {
    pub fn new() -> Self {
        Self {
            player_mask: 0,
            board_mask: 0,
            static_masks: StaticMasks::new(),
            num_moves: 0,
        }
    }

    pub fn from_str(moves: &str) -> Result<Self> {
        let mut board = Self::new();

        for column_char in moves.chars() {
            match column_char.to_digit(10).map(|c| c as usize) {
                Some(column @ 1..=WIDTH) => {
                    let column = column - 1;
                    if !board.playable(column) {
                        return Err(anyhow!("Invalid move, column {} full", column + 1));
                    }
                    let move_bitmap = (board.board_mask + (1 << column * (HEIGHT + 1)))
                        & BitBoard::column_mask(column);
                    let _ = board.play(move_bitmap);
                }
                _ => return Err(anyhow!("could not parse '{}' as a valid move", column_char)),
            }
        }
        Ok(board)
    }

    fn top_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1) + (HEIGHT - 1))
    }

    fn bottom_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1))
    }

    pub fn column_mask(column: usize) -> u64 {
        ((1 << HEIGHT) - 1) << (column * (HEIGHT + 1))
    }

    pub fn non_losing_moves(&self) -> u64 {
        let mut possible_moves = self.possible_moves();
        let opponent_winning_positions = self.opponent_winning_positions();
        let forced_moves = possible_moves & opponent_winning_positions;

        if forced_moves != 0 {
            // if more than one forced move exists, you can't prevent the opponent winning
            if forced_moves & (forced_moves - 1) != 0 {
                return 0;
            } else {
                possible_moves = forced_moves
            }
        }
        // avoid playing below an opponent's winning move
        possible_moves & !(opponent_winning_positions >> 1)
    }

    fn possible_moves(&self) -> u64 {
        (self.board_mask + self.static_masks.bottom_mask) & self.static_masks.full_board_mask
    }

    // create a bitmap of open squares that complete alignments for the opponent
    fn opponent_winning_positions(&self) -> u64 {
        let opp_mask = self.player_mask ^ self.board_mask;
        self.winning_positions(opp_mask)
    }

    fn winning_positions(&self, player_mask: u64) -> u64 {
        // vertical
        // find the top ends of 3-alignemnts
        let mut r = (player_mask << 1) & (player_mask << 2) & (player_mask << 3);

        // horizontal
        let mut p = (player_mask << (HEIGHT + 1)) & (player_mask << 2 * (HEIGHT + 1));
        // find the right ends of 3-alignments
        r |= p & (player_mask << 3 * (HEIGHT + 1));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT + 1));

        p = (player_mask >> (HEIGHT + 1)) & (player_mask >> 2 * (HEIGHT + 1));
        // find the left ends of 3-alignments
        r |= p & (player_mask >> 3 * (HEIGHT + 1));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT + 1));

        // diagonal /
        p = (player_mask << HEIGHT) & (player_mask << 2 * HEIGHT);
        // find the right ends of 3-alignments
        r |= p & (player_mask << 3 * (HEIGHT));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT));

        p = (player_mask >> (HEIGHT)) & (player_mask >> 2 * HEIGHT);
        // find the left ends of 3-alignments
        r |= p & (player_mask >> 3 * (HEIGHT));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT));

        // diagonal \
        p = (player_mask << (HEIGHT + 2)) & (player_mask << 2 * (HEIGHT + 2));
        // find the right ends of 3-alignments
        r |= p & (player_mask << 3 * (HEIGHT + 2));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT + 2));

        p = (player_mask >> (HEIGHT + 2)) & (player_mask >> 2 * (HEIGHT + 2));
        // find the left ends of 3-alignments
        r |= p & (player_mask >> 3 * (HEIGHT + 2));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT + 2));

        r & (self.static_masks.full_board_mask ^ self.board_mask)
    }

    pub fn move_score(&self, candidate: u64) -> i32 {
        // how many open ends of 3-alignments are there?
        self.winning_positions(self.player_mask | candidate)
            .count_ones() as i32
    }

    pub fn num_moves(&self) -> usize {
        self.num_moves
    }
    pub fn playable(&self, column: usize) -> bool {
        Self::top_mask(column) & self.board_mask == 0
    }
    pub fn play(&mut self, move_bitmap: u64) {
        // switch the current player
        self.player_mask ^= self.board_mask;
        // add a cell of the previous player to the correct column
        self.board_mask |= move_bitmap;
        self.num_moves += 1;
    }
    pub fn check_winning_move(&self, column: usize) -> bool {
        let mut pos = self.player_mask.clone();
        // play the move on the clone of the board, keeping the current player
        pos |= (self.board_mask + Self::bottom_mask(column)) & Self::column_mask(column);

        // check horizontal alignment
        // mark all horizontal runs of 2
        let mut m = pos & (pos >> (HEIGHT + 1));
        // check for runs of 2 * (runs of 2)
        if m & (m >> (2 * (HEIGHT + 1))) != 0 {
            return true;
        }

        // check diagonal alignment 1
        // mark all diagonal runs of 2
        m = pos & (pos >> HEIGHT);
        // check for runs of 2 * (runs of 2)
        if m & (m >> (2 * HEIGHT)) != 0 {
            return true;
        }

        // check diagonal alignment 2
        // mark all horizontal runs of 2
        m = pos & (pos >> (HEIGHT + 2));
        // check for runs of 2 * (runs of 2)
        if m & (m >> (2 * (HEIGHT + 2))) != 0 {
            return true;
        }

        // check vertical alignment
        // mark all vertical runs of 2
        m = pos & (pos >> 1);
        // check for runs of 2 * (runs of 2)
        if m & (m >> 2) != 0 {
            return true;
        }

        // no alignments
        false
    }

    pub fn key(&self) -> u64 {
        self.player_mask + self.board_mask
    }
}