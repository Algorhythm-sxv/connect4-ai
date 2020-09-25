//! A perfect agent for playing or analysing the board game 'Connect 4'
//!
//! This agent uses an optimised game tree search to find the 
//! mathematically optimal move for any position.
//!
//! # Basic Usage
//! 
//! ```
//! use connect4_ai::{solver::Solver, bitboard::BitBoard};
//!
//!# use std::error::Error;
//!# fn main() -> Result<(), Box<dyn Error>> {
//! let mut solver = Solver::new(BitBoard::from_moves("112233")?);
//! let (score, best_move) = solver.solve();
//!
//! assert!((score, best_move) == (18, 3));
//!# Ok(())
//!# }
//! ```

use static_assertions::*;
pub use anyhow;

pub mod transposition_table;

pub mod bitboard;

pub mod opening_database;

pub mod solver;

mod test;

/// The width of the game board in tiles
pub const WIDTH: usize = 7;

/// The height of the game board in tiles
pub const HEIGHT: usize = 6;

// ensure that the given dimensions fit in a u64 for the bitboard representation
const_assert!(WIDTH * (HEIGHT + 1) < 64);
