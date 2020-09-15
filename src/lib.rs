mod transposition_table;
pub use transposition_table::*;

mod bitboard;
pub use bitboard::*;

mod arrayboard;
pub use arrayboard::*;

mod opening_database;
pub use opening_database::*;

mod test;


pub const WIDTH: usize = 7;
pub const HEIGHT: usize = 6;
pub const MIN_SCORE: i32 = -((WIDTH * HEIGHT) as i32) / 2 + 3;
pub const MAX_SCORE: i32 = ((WIDTH * HEIGHT) as i32 + 1) / 2 - 3;


struct MoveSorterColumn {
    size: usize,
    // move bitmap, column and score
    moves: [(u64, usize, i32); WIDTH],
}

impl MoveSorterColumn {
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
impl Iterator for MoveSorterColumn {
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
struct MoveSorter {
    size: usize,
    // move bitmap, score
    moves: [(u64, i32); WIDTH],
}

impl MoveSorter {
    pub fn new() -> Self {
        Self {
            size: 0,
            moves: [(0, 0); WIDTH],
        }
    }
    pub fn push(&mut self, new_move: u64, score: i32) {
        let mut pos = self.size;
        self.size += 1;
        while pos != 0 && self.moves[pos - 1].1 > score {
            self.moves[pos] = self.moves[pos - 1];
            pos -= 1;
        }
        self.moves[pos] = (new_move, score);
    }
}
impl Iterator for MoveSorter {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        match self.size {
            0 => None,
            _ => {
                self.size -= 1;
                Some(self.moves[self.size].0)
            }
        }
    }
}

#[derive(Clone)]
pub struct Solver {
    board: BitBoard,
    pub node_count: usize,
    move_order: [usize; WIDTH],
    pub transposition_table: TranspositionTable,
    opening_database: Option<OpeningDatabase>,
}

impl Solver {
    pub fn new(board: BitBoard, opening_database: Option<OpeningDatabase>) -> Self {
        let mut move_order = [0; WIDTH];
        for i in 0..WIDTH {
            move_order[i] = (WIDTH / 2) + (i % 2) * (i / 2 + 1) - (1 - i % 2) * (i / 2);
        }
        Self {
            board,
            node_count: 0,
            move_order,
            transposition_table: TranspositionTable::new(),
            opening_database
        }
    }

    pub fn negamax(&mut self, mut alpha: i32, mut beta: i32) -> i32 {
        self.node_count += 1;

        // look for moves that don't give the opponent a next turn win
        let non_losing_moves = self.board.non_losing_moves();
        if non_losing_moves == 0 {
            return -((WIDTH * HEIGHT) as i32 - self.board.num_moves() as i32) / 2;
        }

        // check for draw
        if self.board.num_moves() == WIDTH * HEIGHT {
            return 0;
        }

        // check for next-move win for current player
        for column in 0..WIDTH {
            if self.board.playable(column) && self.board.check_winning_move(column) {
                return ((WIDTH * HEIGHT + 1 - self.board.num_moves()) / 2) as i32;
            }
        }

        // check opening table at depth 12
        if self.board.num_moves() == 12 {
            if let Some(database) = &self.opening_database {
                let book_score = database.get(self.board.huffman_code(), self.board.huffman_code_mirror());
                if book_score == 33 {
                    panic!("broken score for position '{:032b}', mirror '{:032b}'", self.board.huffman_code(), self.board.huffman_code_mirror());
                } else {
                    return book_score
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
        for i in (0..WIDTH).rev() {
            let column = self.move_order[i];
            let candidate = non_losing_moves & BitBoard::column_mask(column);
            if candidate != 0 && self.board.playable(column) {
                moves.push(candidate, self.board.move_score(candidate))
            }
        }
        for move_bitmap in moves {
            let mut next = self.clone();
            next.node_count = 0;

            next.board.play(move_bitmap);
            // the search window is flipped for the other player
            let score = -next.negamax(-beta, -alpha);
            self.node_count += next.node_count;
            // if the actual score is better than beta, we can prune the tree
            // because the other player will not pick this branch
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

    // top-level search bypasses transposition table and returns the best calculated move
    pub fn top_level_search(&mut self, mut alpha: i32, beta: i32) -> (i32, usize) {
        self.node_count += 1;

        // check for draw (no valid moves)
        if self.board.num_moves() == WIDTH * HEIGHT {
            return (0, WIDTH);
        }

        // look for moves that don't give the opponent a next turn win
        let non_losing_moves = self.board.non_losing_moves();
        if non_losing_moves == 0 {
            // return the first legal move found
            let first = (0..WIDTH).find(|&i| self.board.playable(i)).unwrap();
            return (
                -((WIDTH * HEIGHT) as i32 - self.board.num_moves() as i32) / 2,
                first,
            );
        }

        // check for next-move win for current player
        for column in 0..WIDTH {
            if self.board.playable(column) && self.board.check_winning_move(column) {
                return (
                    ((WIDTH * HEIGHT + 1 - self.board.num_moves()) / 2) as i32,
                    column,
                );
            }
        }

        let mut moves = MoveSorterColumn::new();
        for i in (0..WIDTH).rev() {
            let column = self.move_order[i];
            let candidate = non_losing_moves & BitBoard::column_mask(column);
            if candidate != 0 && self.board.playable(column) {
                moves.push(candidate, column, self.board.move_score(candidate))
            }
        }
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

    pub fn solve(&mut self, silent: bool) -> (i32, usize) {
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
            if r <= mid {
                // actual score <= mid
                max = r
            } else {
                // actual score > mid
                min = r;
            }
        }
        (min, next_move)
    }
}
