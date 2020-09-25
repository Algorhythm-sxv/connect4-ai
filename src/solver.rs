//! An agent to solve the game of Connect 4

use crate::{bitboard::*, opening_database::*, transposition_table::*, HEIGHT, WIDTH};

use std::cmp::Ordering;

/// The minimum possible score of a position
pub const MIN_SCORE: i32 = -((WIDTH * HEIGHT) as i32) / 2 + 3;
/// The maximum possible score of a postion
pub const MAX_SCORE: i32 = ((WIDTH * HEIGHT) as i32 + 1) / 2 - 3;

struct MoveSorter {
    size: usize,
    // move bitmap, column and score
    moves: [(u64, usize, i32); WIDTH],
}

impl MoveSorter {
    pub fn new() -> Self {
        Self {
            size: 0,
            moves: [(0, 0, 0); WIDTH],
        }
    }
    pub fn push(&mut self, new_move: u64, column: usize, score: i32) {
        let mut pos = self.size;
        self.size += 1;
        while pos != 0 && self.moves[pos - 1].2 > score {
            self.moves[pos] = self.moves[pos - 1];
            pos -= 1;
        }
        self.moves[pos] = (new_move, column, score);
    }
}
impl Iterator for MoveSorter {
    type Item = (u64, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self.size {
            0 => None,
            _ => {
                self.size -= 1;
                Some((self.moves[self.size].0, self.moves[self.size].1))
            }
        }
    }
}

/// Returns a slice ordering the columns from the middle outwards, as
/// the middle columns are often better moves
pub const fn move_order() -> [usize; WIDTH] {
    let mut move_order = [0; WIDTH];
    let mut i = 0;
    while i < WIDTH {
        move_order[i] = (WIDTH / 2) + (i % 2) * (i / 2 + 1) - (1 - i % 2) * (i / 2);
        i += 1;
    }
    move_order
}

/// An agent to solve Connect 4 positions
///
/// # Notes
/// This agent uses a classical game tree search with various optimisations to
/// find the mathematically best move(s) in any position, thus 'solving' the game
///
/// # Position Scoring
/// A position is scored by how far a forced win is from the start of the game for either player.
/// If the first player wins with their final placed tile (their 21st tile in a 7x6 board)
/// the score is 1, or -1 if the the second player wins with their final tile. Earlier wins
/// have scores further from 0, up to 18/-18, where a player wins with their 4th tile. A drawn position
/// has a score of 0
#[derive(Clone)]
pub struct Solver {
    board: BitBoard,
    
    /// The number of nodes searched by this `Solver` so far (for diagnostics only)
    pub node_count: usize,
    transposition_table: TranspositionTable,
    opening_database: Option<OpeningDatabase>,
}

impl Solver {

    /// Creates a new `Solver` from a bitboard
    pub fn new(board: BitBoard) -> Self {
        Self {
            board,
            node_count: 0,
            transposition_table: TranspositionTable::new(),
            opening_database: None,
        }
    }

    /// Creates a new `Solver` from a bitboard with a given transposition table
    pub fn new_with_transposition_table(
        board: BitBoard,
        transposition_table: TranspositionTable,
    ) -> Self {
        Self {
            board,
            node_count: 0,
            transposition_table,
            opening_database: None,
        }
    }

    /// Adds an opening database to an existing `Solver`
    pub fn with_opening_database(mut self, opening_database: OpeningDatabase) -> Self {
        self.opening_database = Some(opening_database);
        self
    }

    /// Performs game tree search
    ///
    /// Returns the score of the position (see [Position Scoring])
    ///
    /// [Position Scoring]: #position-scoring
    fn negamax(&mut self, mut alpha: i32, mut beta: i32) -> i32 {
        self.node_count += 1;

        // check for next-move win for current player
        for column in 0..WIDTH {
            if self.board.playable(column) && self.board.check_winning_move(column) {
                return ((WIDTH * HEIGHT + 1 - self.board.num_moves()) / 2) as i32;
            }
        }

        // look for moves that don't give the opponent a next turn win
        let non_losing_moves = self.board.non_losing_moves();
        if non_losing_moves == 0 {
            return -((WIDTH * HEIGHT) as i32 - self.board.num_moves() as i32) / 2;
        }

        // check for draw
        if self.board.num_moves() == WIDTH * HEIGHT {
            return 0;
        }

        // check opening table at appropriate depth
        if self.board.num_moves() == DATABASE_DEPTH {
            if let Some(database) = &self.opening_database {
                if let Some(score) = database.get(self.board.huffman_code()) {
                    return score;
                }
            }
        }

        // upper bound of score
        let mut max = (((WIDTH * HEIGHT) - 1 - self.board.num_moves()) / 2) as i32;

        // try to fetch the upper/lower bound of the score from the transposition table
        let key = self.board.key();
        let value = self.transposition_table.get(key) as i32;
        if value != 0 {
            // check if lower bound
            if value > MAX_SCORE - MIN_SCORE + 1 {
                let min = value + 2 * MIN_SCORE - MAX_SCORE - 2;
                if alpha < min {
                    alpha = min;
                    if alpha >= beta {
                        // prune the exploration
                        return alpha;
                    }
                }
            // else upper bound
            } else {
                let max = value + MIN_SCORE - 1;
                if beta > max {
                    beta = max;
                    if alpha >= beta {
                        // prune the exploration
                        return beta;
                    }
                }
            }
            max = value + MIN_SCORE - 1;
        }
        if beta > max {
            // clamp beta to calculated upper bound
            beta = max;
            // if the upper bound is lower than alpha, we can prune the exploration
            if alpha >= beta {
                return beta;
            };
        }

        let mut moves = MoveSorter::new();
        // reversing move order to put edges first reduces the amount of sorting
        // as these moves are worse on average
        for i in (0..WIDTH).rev() {
            let column = move_order()[i];
            let candidate = non_losing_moves & BitBoard::column_mask(column);
            if candidate != 0 && self.board.playable(column) {
                moves.push(candidate, column, self.board.move_score(candidate))
            }
        }

        // search the next level of the tree
        for (move_bitmap, _column) in moves {
            let mut next = self.clone();
            next.node_count = 0;

            next.board.play(move_bitmap);
            // the search window is flipped for the other player
            let score = -next.negamax(-beta, -alpha);
            self.node_count += next.node_count;
            // if a child node's score is better than beta, we can prune the tree
            // here because a perfect opponent will not pick this branch
            if score >= beta {
                // save a lower bound of the score
                self.transposition_table
                    .set(key, (score + MAX_SCORE - 2 * MIN_SCORE + 2) as u8);
                return score;
            }
            if score > alpha {
                alpha = score;
            }
        }

        // offset of one to prevent putting a 0, which represents an empty entry
        self.transposition_table
            .set(self.board.key(), (alpha - MIN_SCORE + 1) as u8);
        alpha
    }

    /// Performs a top-level search, bypassing transposition table and opening database
    ///
    /// Returns the score of the position and the calculated best move
    fn top_level_search(&mut self, mut alpha: i32, beta: i32) -> (i32, usize) {
        self.node_count += 1;

        // check for win for current player on this move
        for column in 0..WIDTH {
            if self.board.playable(column) && self.board.check_winning_move(column) {
                return (
                    ((WIDTH * HEIGHT + 1 - self.board.num_moves()) / 2) as i32,
                    column,
                );
            }
        }

        // look for moves that don't give the opponent a next turn win
        let non_losing_moves = self.board.non_losing_moves();
        if non_losing_moves == 0 {
            // all moves lose, return the first legal move found
            let first = (0..WIDTH).find(|&i| self.board.playable(i)).unwrap();
            return (
                -((WIDTH * HEIGHT) as i32 - self.board.num_moves() as i32) / 2,
                first,
            );
        }

        // check for draw (no valid moves)
        if self.board.num_moves() == WIDTH * HEIGHT {
            return (0, WIDTH);
        }

        let mut moves = MoveSorter::new();
        for i in (0..WIDTH).rev() {
            let column = move_order()[i];
            let candidate = non_losing_moves & BitBoard::column_mask(column);
            if candidate != 0 && self.board.playable(column) {
                moves.push(candidate, column, self.board.move_score(candidate))
            }
        }

        // search the next level of the tree and keep track of the best move
        let mut best_score = MIN_SCORE;
        let mut best_move = WIDTH;
        for (move_bitmap, column) in moves {
            let mut next = self.clone();
            next.node_count = 0;

            next.board.play(move_bitmap);
            // the search window is flipped for the other player
            let score = -next.negamax(-beta, -alpha);
            self.node_count += next.node_count;
            // if the actual score is better than beta, we can prune the tree
            // because the other player will not pick this branch
            if score >= beta {
                return (score, column);
            }
            if score > alpha {
                alpha = score;
            }
            if score > best_score {
                best_score = score;
                best_move = column;
            }
        }

        (alpha, best_move)
    }

    /// Calculate the score and best move of the current position with iterative deepening
    pub fn solve(&mut self) -> (i32, usize) {
        self._solve(true)
    }
    
    /// Calculate the score and best move of the current position with iterative deepening, logging progress to stdout
    pub fn solve_verbose(&mut self) -> (i32, usize) {
        self._solve(false)
    }

    /// Performs the iterative deepening search, returning position score and best move
    fn _solve(&mut self, silent: bool) -> (i32, usize) {
        let mut min = -(((WIDTH * HEIGHT) as i32) - self.board.num_moves() as i32) / 2;
        let mut max = (WIDTH * HEIGHT + 1 - self.board.num_moves()) as i32 / 2;

        let mut next_move = WIDTH;
        // iteratively narrow the search window for iterative deepening
        while min < max {
            let mut mid = min + (max - min) / 2;
            // tweak the search value for both negative and positive searches
            if mid <= 0 && min / 2 < mid {
                mid = min / 2
            } else if mid >= 0 && max / 2 > mid {
                mid = max / 2
            }

            // log progress to stdout
            if !silent {
                println!(
                    "Search depth: {}/{}, uncertainty: {}",
                    (WIDTH * HEIGHT - self.board.num_moves()) as i32 - min.abs().min(max.abs()),
                    WIDTH * HEIGHT - self.board.num_moves(),
                    max - min
                );
            }

            // use a null-window to determine if the actual score is greater or less that mid
            let (r, best_move) = self.top_level_search(mid, mid + 1);
            next_move = best_move;

            // r is not necessarily the exact true score, but its value indicates
            // whether the true score is above or below the search target
            if r <= mid {
                // actual score <= mid
                max = r
            } else {
                // actual score > mid
                min = r;
            }
        }
        // min and max should be equal here
        (min, next_move)
    }

    /// Converts a position score to a win distance in a single player's moves
    pub fn score_to_win_distance(&self, score: i32) -> usize {
        match score.cmp(&0) {
            Ordering::Equal => WIDTH * HEIGHT - self.board.num_moves(),
            Ordering::Greater => {
                (WIDTH * HEIGHT / 2 + 1 - score as usize) - self.board.num_moves() / 2
            }
            Ordering::Less => {
                (WIDTH * HEIGHT / 2 + 1) - (-score as usize) - self.board.num_moves() / 2
            }
        }
    }
}

impl std::ops::Deref for Solver {
    type Target = BitBoard;

    fn deref(&self) -> &Self::Target {
        &self.board
    }
}
