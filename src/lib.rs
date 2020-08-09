mod transposition_table;
use transposition_table::*;

mod bitboard;
pub use bitboard::*;

mod arrayboard;
pub use arrayboard::*;

use std::sync::{Arc, Mutex};
use std::thread;

const WIDTH: usize = 7;
const HEIGHT: usize = 6;
const MIN_SCORE: i32 = -((WIDTH * HEIGHT) as i32) / 2 + 3;
const MAX_SCORE: i32 = ((WIDTH * HEIGHT) as i32 + 1) / 2 - 3;


struct MoveSorter {
    size: usize,
    // move bitmap and score
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

    fn next(&mut self) -> Option<u64> {
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
    transposition_table: SharedTranspositionTable,
}

impl Solver {
    pub fn new(board: BitBoard) -> Self {
        let mut move_order = [0; WIDTH];
        for i in 0..WIDTH {
            move_order[i] = (WIDTH / 2) + (i % 2) * (i / 2 + 1) - (1 - i % 2) * (i / 2);
        }
        Self {
            board,
            node_count: 0,
            move_order,
            transposition_table: SharedTranspositionTable::new(),
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
            max = value as i32 + MIN_SCORE - 1;
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
            let candidate = non_losing_moves & BitBoard::column_mask(self.move_order[i]);
            if candidate != 0 {
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

    pub fn solve(&mut self) -> i32 {
        let mut min = -(((WIDTH * HEIGHT) as i32) - self.board.num_moves() as i32) / 2;
        let mut max = (WIDTH * HEIGHT + 1 - self.board.num_moves()) as i32 / 2;

        // iteratively narrow the search window
        while min < max {
            let mut mid = min + (max - min) / 2;
            // tweak the search value for both negative and positive searches
            if mid <= 0 && min / 2 < mid {
                mid = min / 2
            } else if mid >= 0 && max / 2 > mid {
                mid = max / 2
            }

            // use a null-window to determine if the actual score is greater or less that mid
            let r = self.negamax(mid, mid + 1);
            if r <= mid {
                // actual score <= mid
                max = r
            } else {
                // actual score > mid
                min = r
            }
        }
        min
    }

    pub fn par_solve(&mut self) -> (i32, usize) {
        // WIDTH will always be an invalid column to play
        let best = Arc::new(Mutex::new((MIN_SCORE, WIDTH)));
        let mut threads = vec![];

        for column in 0..WIDTH {
            if self.board.playable(column) {
                let mut next = self.clone();
                let best = Arc::clone(&best);

                threads.push(thread::spawn(move || {
                    let move_bitmap = (next.board.board_mask + (1 << column * (HEIGHT + 1)))
                        & BitBoard::column_mask(column);
                    next.board.play(move_bitmap);

                    let score = -next.solve();

                    let mut best = best.lock().unwrap();
                    if score > best.0 {
                        *best = (score, column);
                    }
                }));
            }
        }
        for child in threads {
            let _ = child.join();
        }
        Arc::try_unwrap(best).unwrap().into_inner().unwrap()
    }
}

#[cfg(test)]
pub mod test {
    use anyhow::{anyhow, Result};
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::time::{Duration, Instant};

    use crate::{ArrayBoard, BitBoard, Solver};

    #[allow(unused)]
    #[test]
    pub fn test_print() -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        let _ =
            BufReader::new(File::open("test_data/Test_L3_R1")?).read_until(' ' as u8, &mut buf)?;
        buf.pop();

        let board = ArrayBoard::from_str(std::str::from_utf8(&buf)?)?;
        board.display()?;
        Ok(())
    }

    #[test]
    pub fn end_easy() -> Result<()> {
        let file = BufReader::new(File::open("test_data/Test_L3_R1")?);

        let mut times = vec![];
        let mut posis = vec![];

        for line in file.split(b'\n').take(100) {
            let buf = String::from_utf8(line?)?;
            let mut test_data = buf.split_whitespace();
            let moves = test_data.next().ok_or(anyhow!(
                "invalid test data: {}",
                test_data.clone().collect::<String>()
            ))?;
            let score = test_data
                .next()
                .ok_or(anyhow!(
                    "invalid test data: {}",
                    test_data.clone().collect::<String>()
                ))?
                .parse::<i32>()?;

            let board = BitBoard::from_str(moves)?;
            let mut solver = Solver::new(board);
            let start_time = Instant::now();
            let calc = solver.solve();
            let finish_time = Instant::now();
            assert!(score == calc);
            times.push(finish_time - start_time);
            posis.push(solver.node_count);
        }

        println!(
            "End-easy:\nMean time: {:.6}ms, Mean no. of positions: {}, kpos/s: {}",
            (times.iter().sum::<Duration>() / times.len() as u32).as_secs_f64() * 1000.0,
            posis.iter().sum::<usize>() as f64 / posis.len() as f64,
            posis
                .iter()
                .zip(times.iter())
                .map(|(p, t)| *p as f64 / t.as_secs_f64())
                .sum::<f64>()
                / (1000.0 * posis.len() as f64)
        );
        Ok(())
    }

    #[test]
    pub fn middle_easy() -> Result<()> {
        let file = BufReader::new(File::open("test_data/Test_L2_R1")?);

        let mut times = vec![];
        let mut posis = vec![];

        for line in file.split(b'\n').take(100) {
            let buf = String::from_utf8(line?)?;

            let mut test_data = buf.split_whitespace();
            let moves = test_data.next().ok_or(anyhow!(
                "invalid test data: {}",
                test_data.clone().collect::<String>()
            ))?;
            let score = test_data
                .next()
                .ok_or(anyhow!(
                    "invalid test data: {}",
                    test_data.clone().collect::<String>()
                ))?
                .parse::<i32>()?;

            let board = BitBoard::from_str(moves)?;
            let mut solver = Solver::new(board);
            let start_time = Instant::now();
            let calc = solver.solve();
            let finish_time = Instant::now();
            assert!(score == calc);
            times.push(finish_time - start_time);
            posis.push(solver.node_count);
        }

        println!(
            "Middle-easy\nMean time: {:.6}ms, Mean no. of positions: {}, kpos/s: {}",
            (times.iter().sum::<Duration>() / times.len() as u32).as_secs_f64() * 1000.0,
            posis.iter().sum::<usize>() as f64 / posis.len() as f64,
            posis
                .iter()
                .zip(times.iter())
                .map(|(p, t)| *p as f64 / t.as_secs_f64())
                .sum::<f64>()
                / (1000.0 * posis.len() as f64)
        );
        Ok(())
    }

    #[test]
    pub fn middle_medium() -> Result<()> {
        let file = BufReader::new(File::open("test_data/Test_L2_R2")?);

        let mut times = vec![];
        let mut posis = vec![];

        for line in file.split(b'\n').take(20) {
            let buf = String::from_utf8(line?)?;

            let mut test_data = buf.split_whitespace();
            let moves = test_data.next().ok_or(anyhow!(
                "invalid test data: {}",
                test_data.clone().collect::<String>()
            ))?;
            let score = test_data
                .next()
                .ok_or(anyhow!(
                    "invalid test data: {}",
                    test_data.clone().collect::<String>()
                ))?
                .parse::<i32>()?;

            let board = BitBoard::from_str(moves)?;
            let mut solver = Solver::new(board);
            let start_time = Instant::now();
            let calc = solver.solve();
            let finish_time = Instant::now();
            assert!(score == calc);
            times.push(finish_time - start_time);
            posis.push(solver.node_count);
        }

        println!(
            "Middle-medium\nMean time: {:.6}ms, Mean no. of positions: {}, kpos/s: {}",
            (times.iter().sum::<Duration>() / times.len() as u32).as_secs_f64() * 1000.0,
            posis.iter().sum::<usize>() as f64 / posis.len() as f64,
            posis
                .iter()
                .zip(times.iter())
                .map(|(p, t)| *p as f64 / t.as_secs_f64())
                .sum::<f64>()
                / (1000.0 * posis.len() as f64)
        );
        Ok(())
    }

    #[test]
    pub fn begin_hard() -> Result<()> {
        let file = BufReader::new(File::open("test_data/Test_L1_R3")?);

        let mut times = vec![];
        let mut posis = vec![];

        for line in file.split(b'\n').take(1) {
            let buf = String::from_utf8(line?)?;

            let mut test_data = buf.split_whitespace();
            let moves = test_data.next().ok_or(anyhow!(
                "invalid test data: {}",
                test_data.clone().collect::<String>()
            ))?;
            let score = test_data
                .next()
                .ok_or(anyhow!(
                    "invalid test data: {}",
                    test_data.clone().collect::<String>()
                ))?
                .parse::<i32>()?;

            let board = BitBoard::from_str(moves)?;
            let mut solver = Solver::new(board);
            let start_time = Instant::now();
            let calc = solver.solve();
            let finish_time = Instant::now();
            assert!(score == calc);
            times.push(finish_time - start_time);
            posis.push(solver.node_count);
        }

        println!(
            "Beginning-Hard\nMean time: {:.6}ms, Mean no. of positions: {}, kpos/s: {}",
            (times.iter().sum::<Duration>() / times.len() as u32).as_secs_f64() * 1000.0,
            posis.iter().sum::<usize>() as f64 / posis.len() as f64,
            posis
                .iter()
                .zip(times.iter())
                .map(|(p, t)| *p as f64 / t.as_secs_f64())
                .sum::<f64>()
                / (1000.0 * posis.len() as f64)
        );
        Ok(())
    }
}
