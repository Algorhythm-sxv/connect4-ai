use anyhow::{anyhow, Result};

use crate::{HEIGHT, WIDTH};

mod static_masks {
    use crate::{HEIGHT, WIDTH};

    pub const fn bottom_mask() -> u64 {
        let mut mask = 0;
        let mut column = 0;
        while column < WIDTH {
            mask |= 1 << (column * (HEIGHT + 1));
            column += 1;
        }
        mask
    }
    pub const fn full_board_mask() -> u64 {
        bottom_mask() * ((1 << HEIGHT as u64) - 1)
    }
}
#[derive(Copy, Clone)]
pub struct BitBoard {
    // mask of the current player's tiles
    player_mask: u64,
    // mask of all tiles
    board_mask: u64,
    num_moves: usize,
}
impl BitBoard {
    pub fn new() -> Self {
        Self {
            player_mask: 0,
            board_mask: 0,
            num_moves: 0,
        }
    }

    pub fn from_moves<S: AsRef<str>>(moves: S) -> Result<Self> {
        let mut board = Self::new();

        for column_char in moves.as_ref().chars() {
            // only play available moves
            match column_char.to_digit(10).map(|c| c as usize) {
                Some(column @ 1..=WIDTH) => {
                    let column = column - 1;
                    if !board.playable(column) {
                        return Err(anyhow!("Invalid move, column {} full", column + 1));
                    }
                    // abort if the position is won at any point
                    if board.check_winning_move(column) {
                        return Err(anyhow!("Invalid position, game is over"));
                    }
                    let move_bitmap = (board.board_mask + (1 << (column * (HEIGHT + 1))))
                        & BitBoard::column_mask(column);
                    board.play(move_bitmap);
                }
                _ => return Err(anyhow!("could not parse '{}' as a valid move", column_char)),
            }
        }
        Ok(board)
    }

    // for database generation, assumes all moves are in range
    pub fn from_slice(moves: &[usize]) -> Result<Self, ()> {
        let mut board = Self::new();
        for &column in moves.iter() {
            if !board.playable(column) {
                return Err(());
            }
            // abort if the position is won at any point
            if board.check_winning_move(column) {
                return Err(());
            }
            let move_bitmap =
                (board.board_mask + (1 << (column * (HEIGHT + 1)))) & BitBoard::column_mask(column);
            board.play(move_bitmap);
        }
        Ok(board)
    }

    pub fn from_masks(player_mask: u64, board_mask: u64, num_moves: usize) -> Self {
        Self {
            player_mask,
            board_mask,
            num_moves,
        }
    }

    pub fn player_mask(&self) -> u64 {
        self.player_mask
    }

    pub fn board_mask(&self) -> u64 {
        self.board_mask
    }

    pub fn top_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1) + (HEIGHT - 1))
    }

    pub fn bottom_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1))
    }

    pub fn column_mask(column: usize) -> u64 {
        ((1 << HEIGHT) - 1) << (column * (HEIGHT + 1))
    }
    pub fn column_from_move(move_bitmap: u64) -> usize {
        for column in 0..WIDTH {
            if move_bitmap & Self::column_mask(column) != 0 {
                return column;
            }
        }
        // WIDTH is always an invalid column
        WIDTH
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

    pub fn possible_moves(&self) -> u64 {
        (self.board_mask + static_masks::bottom_mask()) & static_masks::full_board_mask()
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
        let mut p = (player_mask << (HEIGHT + 1)) & (player_mask << (2 * (HEIGHT + 1)));
        // find the right ends of 3-alignments
        r |= p & (player_mask << (3 * (HEIGHT + 1)));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT + 1));

        p = (player_mask >> (HEIGHT + 1)) & (player_mask >> (2 * (HEIGHT + 1)));
        // find the left ends of 3-alignments
        r |= p & (player_mask >> (3 * (HEIGHT + 1)));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT + 1));

        // diagonal /
        p = (player_mask << HEIGHT) & (player_mask << (2 * HEIGHT));
        // find the right ends of 3-alignments
        r |= p & (player_mask << (3 * (HEIGHT)));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT));

        p = (player_mask >> (HEIGHT)) & (player_mask >> (2 * HEIGHT));
        // find the left ends of 3-alignments
        r |= p & (player_mask >> (3 * (HEIGHT)));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT));

        // diagonal \
        p = (player_mask << (HEIGHT + 2)) & (player_mask << (2 * (HEIGHT + 2)));
        // find the right ends of 3-alignments
        r |= p & (player_mask << (3 * (HEIGHT + 2)));
        // find holes of the type ...O O _ O...
        r |= p & (player_mask >> (HEIGHT + 2));

        p = (player_mask >> (HEIGHT + 2)) & (player_mask >> (2 * (HEIGHT + 2)));
        // find the left ends of 3-alignments
        r |= p & (player_mask >> (3 * (HEIGHT + 2)));
        // find holes of the type ...O _ O O...
        r |= p & (player_mask << (HEIGHT + 2));

        r & (static_masks::full_board_mask() ^ self.board_mask)
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
        let mut pos = self.player_mask;
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

    // key for transposition table
    pub fn key(&self) -> u64 {
        self.player_mask + self.board_mask
    }

    pub fn huffman_code(&self) -> u32 {
        self._huffman_code(false)
    }
    pub fn huffman_code_mirror(&self) -> u32 {
        self._huffman_code(true)
    }
    // Huffman code for opening database
    fn _huffman_code(&self, mirror: bool) -> u32 {
        // 0 separates the tiles of each column
        // 10 is a player 1 tile
        // 11 is a player 2 tile
        let mut code = 0;

        let iter: Box<dyn Iterator<Item = usize>> = if mirror {
            Box::new((0..WIDTH).rev())
        } else {
            Box::new(0..WIDTH)
        };
        for column in iter {
            let column_mask = Self::column_mask(column);
            // go over the top of the columns to add a separator when a row is full
            for row in 0..=HEIGHT {
                let row_mask = static_masks::bottom_mask() << row;
                let tile_mask = column_mask & row_mask;

                // end of column
                if self.board_mask & tile_mask == 0 {
                    // append 0
                    code <<= 1;
                    break;
                // tile present
                } else {
                    // player 1 tile
                    if self.player_mask & tile_mask != 0 {
                        // append 10
                        code = (code << 2) + 0b10;
                    // player 2 tile
                    } else {
                        // append 11
                        code = (code << 2) + 0b11;
                    }
                }
            }
        }
        code << 1
    }
}

impl Default for BitBoard {
    fn default() -> Self {
        Self::new()
    }
}
