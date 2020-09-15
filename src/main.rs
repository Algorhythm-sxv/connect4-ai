use connect4_ai::*;

use anyhow::Result;

use std::io::{stdin, stdout, Write};
fn main() -> Result<()> {
    let mut board = ArrayBoard::new();
    let transposition_table = TranspositionTable::new();

    let stdin = stdin();
    println!("Welcome to Connect 4\n");

    let mut opening_database: Option<OpeningDatabase> = None;
    let opening_database_result = OpeningDatabase::load();
    match opening_database_result {
        Ok(database) => {opening_database = Some(database);}
        Err(err) => {
            for cause in err.chain() {
                if let Some(io_error) = cause.downcast_ref::<std::io::Error>() {
                    match io_error.kind() {
                        std::io::ErrorKind::NotFound => {
                            println!("Opening database not found, expect early AI moves to take ~10 minutes");
                        }
                        _ => println!("Error reading opening database: {}", io_error),
                    }
                } else {
                    println!("{}", err);
                }
            }
        }
    }

    loop {
        board.display().expect("Failed to draw board!");

        match board.state {
            GameState::Playing => {
                let next_move = if board.player_one {
                    println!("AI is thinking...");
                    stdout().flush().expect("Failed to flush to stdout!");

                    let mut solver = Solver::new(BitBoard::from_str(&board.game)?, opening_database.clone());
                    solver.transposition_table = transposition_table.clone();
                    let (_score, best_move) = solver.solve(false);
                    println!("Best: {}", best_move + 1);
                    best_move + 1
                } else {
                    print!("> ");
                    stdout().flush().expect("Failed to flush to stdout!");
                    let mut input_str = String::new();
                    stdin
                        .read_line(&mut input_str)
                        .expect("Failed to read stdin!");

                    match input_str.trim().parse::<usize>() {
                        Err(_) => {
                            println!("Invalid number: {}", input_str);
                            continue;
                        }
                        Ok(column) => column,
                    }
                };

                match board.play_checked(next_move) {
                    Err(err) => {
                        println!("{}", err);
                        continue;
                    }
                    Ok(_) => {}
                }
            }
            GameState::PlayerOneWin => {
                board.display().expect("Failed to draw board!");
                println!("Player 1 wins!");
                break;
            }
            GameState::PlayerTwoWin => {
                board.display().expect("Failed to draw board!");
                println!("Player 2 wins!");
                break;
            }
            GameState::Draw => {
                board.display().expect("Failed to draw board!");
                println!("Draw!");
                break;
            }
        }
    }
    Ok(())
}
