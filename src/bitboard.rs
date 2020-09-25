//! A compact, computationally efficient bit array representation of a Connect 4 board 

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

/// A Connect 4 bitboard
///
/// # Notes
/// Storing the state of the board in the bits of an integer allows parallel
/// computation of game conditions with bitwise operations. A 7x6 Connect 4
/// board fits into the bits of a `u64` like so:
/// 
/// ```comment
/// Column:  0  1  2  3  4  5  6
///
///          6  13 20 28 35 42 49
///          ____________________
///       5 |05 12 19 27 34 41 48|
///       4 |04 11 18 26 33 40 47|
///       3 |03 10 17 24 32 39 46|
///       2 |02 09 16 23 31 38 45|
///       1 |01 08 15 22 30 37 44|
/// Rows: 0 |00 07 14 21 29 36 43|
/// ```
/// Where bit index 00 is the least significant bit. The extra row of bits on top of the board
/// identifies full columns and prevents bits overflowing into the next column
///
/// # Board Keys
/// A Connect 4 board can be unambiguously represented in a single u64 by placing a 1-bit in
/// each square the board where the current player has a tile, and an additional 1-bit in
/// the first empty square of a column. This representation is used to index the [transposition table]
/// and created by [`BitBoard::key`]
///
/// # Internal Representation
/// This bitboard uses 2 `u64`s for computational efficiency. One `u64` stores a mask of all squares
/// containing a tile of either color, and the other stores a mask of the current player's tiles
///
/// # Huffman Codes
/// A board with up to 12 tiles can be encoded into a `u32` using a 
/// [Huffman code](https://en.wikipedia.org/wiki/Huffman_coding), where the bit sequence `0` separates each 
/// column and the code sequences `10` and `11` represent the first and second player's tiles respectively.
/// A board with 12 tiles requires 6 bits of separators and 24 bits of tiles, for 30 bits total
///
/// [transposition table]: ../transposition_table/struct.TranspositionTable.html
/// [`BitBoard::key`]: #method.key
#[derive(Copy, Clone)]
pub struct BitBoard {
    // mask of the current player's tiles
    player_mask: u64,
    // mask of all tiles
    board_mask: u64,
    num_moves: usize,
}
impl BitBoard {
    /// Creates a new, empty bitboard
    pub fn new() -> Self {
        Self {
            player_mask: 0,
            board_mask: 0,
            num_moves: 0,
        }
    }

    /// Creates a board from a string of 1-indexed moves
    /// 
    /// # Notes
    /// The move string is a sequence of columns played, indexed from 1 (meaning `"0"` is an invalid move)
    /// 
    /// Returns `Err` if the move string represents an invalid position. Invalid positions can contain moves
    /// outside the column range, overfilled columns and winning positions for either player
    ///
    /// # Example
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use connect4_ai::bitboard::BitBoard;
    ///
    /// // columns in move strings are 1-indexed
    /// let board = BitBoard::from_moves("112233")?;
    /// 
    /// // columns as integers are 0-indexed
    /// assert!(board.check_winning_move(3));
    /// # Ok(())
    /// # }
    /// ```
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

    /// Creates a board from a slice of 0-indexed moves
    /// 
    /// Significantly faster than [`BitBoard::from_moves`] but provides less informative errors
    ///
    /// Returns `Err` if the board position is invalid (see [`BitBoard::from_moves`])
    ///
    /// # Warning
    /// This method assumes all items in the slice are in the valid column range, providing numbers too large
    /// can cause a `panic` in debug builds by bit-shift overflow or produce an unexpected bitboard
    /// 
    /// # Example
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), ()> {
    /// use connect4_ai::bitboard::BitBoard;
    ///
    /// let board = BitBoard::from_slice(&[0, 0, 1, 1, 2, 2])?;
    /// 
    /// assert!(board.check_winning_move(3));
    /// # Ok(())
    /// # }
    /// ```
    /// [`BitBoard::from_moves`]: #method.from_moves
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

    /// Creates a bitboard from its constituent bit masks and move counter (see [Internal Representation])
    /// [Internal Representation]: #internal-representation
    pub fn from_parts(player_mask: u64, board_mask: u64, num_moves: usize) -> Self {
        Self {
            player_mask,
            board_mask,
            num_moves,
        }
    }

    /// Accesses the internal mask of the current player's tiles
    pub fn player_mask(&self) -> u64 {
        self.player_mask
    }

    /// Accesses the internal mask of tiles on the whole board
    pub fn board_mask(&self) -> u64 {
        self.board_mask
    }

    /// Returns a mask of the top square of a given column
    pub fn top_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1) + (HEIGHT - 1))
    }

    /// Returns a mask of the bottom square of a given column
    pub fn bottom_mask(column: usize) -> u64 {
        1 << (column * (HEIGHT + 1))
    }

    /// Returns a mask of the given column
    pub fn column_mask(column: usize) -> u64 {
        ((1 << HEIGHT) - 1) << (column * (HEIGHT + 1))
    }

    /// Returns the column represented by a move bitmap or [`WIDTH`] if the column is not found
    ///
    /// [`WIDTH`]: ../constant.WIDTH.html
    pub fn column_from_move(move_bitmap: u64) -> usize {
        for column in 0..WIDTH {
            if move_bitmap & Self::column_mask(column) != 0 {
                return column;
            }
        }
        // WIDTH is always an invalid column
        WIDTH
    }

    /// Returns a bitmap of all moves that don't give the opponent an immediate win
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

    /// Returns a mask of all possible moves in the position
    pub fn possible_moves(&self) -> u64 {
        (self.board_mask + static_masks::bottom_mask()) & static_masks::full_board_mask()
    }

    /// Returns a bitmap of open squares that complete alignments for the opponent
    fn opponent_winning_positions(&self) -> u64 {
        let opp_mask = self.player_mask ^ self.board_mask;
        self.winning_positions(opp_mask)
    }

    /// Returns a mask of open squares of the current player's partial alignments
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

    /// Scores a move bitmap by counting open 3-alignments after the move
    pub fn move_score(&self, candidate: u64) -> i32 {
        // how many open ends of 3-alignments are there?
        self.winning_positions(self.player_mask | candidate)
            .count_ones() as i32
    }

    /// Accesses the internal move counter
    pub fn num_moves(&self) -> usize {
        self.num_moves
    }

    /// Returns whether a column is a legal move
    pub fn playable(&self, column: usize) -> bool {
        Self::top_mask(column) & self.board_mask == 0
    }

    /// Advances the game by applying a move bitmap and switching players
    pub fn play(&mut self, move_bitmap: u64) {
        // switch the current player
        self.player_mask ^= self.board_mask;
        // add a cell of the previous player to the correct column
        self.board_mask |= move_bitmap;
        self.num_moves += 1;
    }

    /// Returns whether a column is a winning move
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

    /// Returns the key used for indexing into the transposition table (see [Board Keys])
    ///
    /// [Board Keys]: #board-keys
    pub fn key(&self) -> u64 {
        self.player_mask + self.board_mask
    }

    /// Returns the Huffman code used for searching the opening database (see [Huffman Codes])
    /// 
    /// # Notes
    /// For positions with more than 13 tiles, data will be lost and the returned code will not
    /// be unique
    ///
    /// [Huffman Codes]: #huffman-codes
    pub fn huffman_code(&self) -> u32 {
        self._huffman_code(false).min(self._huffman_code(true))
    }

    /// Returns Huffman code for opening database, optionally mirroring the position
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
