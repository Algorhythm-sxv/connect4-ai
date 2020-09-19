use anyhow::Result;

use std::cmp::Ordering;
use std::io::{stdin, stdout, Write};

use connect4_ai::*;

mod arrayboard;
use arrayboard::*;

fn main() -> Result<()> {
    let mut board = ArrayBoard::new();
    // keep the transposition table out here so we can re-use it
    let transposition_table = TranspositionTable::new();

    let stdin = stdin();

    println!("Welcome to Connect 4\n");

    // check for opening database
    let mut opening_database: Option<OpeningDatabase> = None;
    let opening_database_result = OpeningDatabase::load();
    match opening_database_result {
        Ok(database) => {
            opening_database = Some(database);
        }
        Err(err) => match err.root_cause().downcast_ref::<std::io::Error>() {
            Some(io_error) => if let std::io::ErrorKind::NotFound = io_error.kind() {
                loop {
                    print!(
                        "Opening database not found, would you like to generate one? (takes a LONG time)\ny/n: "
                    );
                    stdout().flush().expect("failed to flush to stdout!");

                    let mut buffer = String::new();
                    stdin.read_line(&mut buffer)?;

                    match buffer.to_lowercase().chars().next() {
                        Some(_letter @ 'y') => {
                            OpeningDatabase::generate()?;
                            return Ok(());
                            // break;
                        },
                        Some(_letter @ 'n') => {
                            println!("Skipping database generation, expect early AI moves to take ~10 minutes");
                            break;
                        },
                        _ => println!("Unknown answer given"),
                    }
                }
            } else {
                println!("Error reading opening database: {}", err.root_cause());
            },
            _ => println!("Error reading opening database: {}", err.root_cause()),
        },
    }

    let mut ai_players = (false, false);

    // choose AI control of player 1
    loop {
        let mut buffer = String::new();
        print!("Is player 1 AI controlled? y/n: ");
        stdout().flush().expect("failed to flush to stdout!");
        stdin.read_line(&mut buffer)?;
        match buffer.to_lowercase().chars().next() {
            Some(_letter @ 'y') => {
                ai_players.0 = true;
                break;
            }
            Some(_letter @ 'n') => break,
            _ => println!("Unknown answer given"),
        }
    }

    // choose AI control of player 2
    loop {
        let mut buffer = String::new();
        print!("Is player 2 AI controlled? y/n: ");
        stdout().flush().expect("failed to flush to stdout!");
        stdin.read_line(&mut buffer)?;
        match buffer.to_lowercase().chars().next() {
            Some(_letter @ 'y') => {
                ai_players.1 = true;
                break;
            },
            Some(_letter @ 'n') => break,
            _ => println!("Unknown answer given"),
        }
    }

    // game loop
    loop {
        board.display().expect("Failed to draw board!");

        match board.state {
            GameState::Playing => {
                let next_move =
                    // AI player
                    if (board.player_one && ai_players.0) || (!board.player_one && ai_players.1) {
                        println!("AI is thinking...");
                        stdout().flush().expect("Failed to flush to stdout!");

                        // slow down play if both players are AI
                        if ai_players == (true, true) {
                            std::thread::sleep(std::time::Duration::new(3, 0));
                        }

                        let mut solver = Solver::new_with_transposition_table(
                            BitBoard::from_moves(&board.game)?,
                            transposition_table.clone(),
                        );
                        if let Some(database) = opening_database.clone() {
                            solver = solver.with_opening_database(database);
                        }

                        let (score, best_move) = solver.solve();

                        let win_distance = solver.score_to_win_distance(score);
                        let move_string = if win_distance == 1 {"move"} else {"moves"};
                        match score.cmp(&0) {
                            Ordering::Greater =>  {
                                let player = if board.player_one { 1 } else { 2 };
                                println!("Player {} can force a win in at most {} {}.", player, win_distance, move_string);
                            },
                            Ordering::Less => {
                                let player = if board.player_one { 2 } else { 1 };
                                println!("Player {} can force a win in at most {} {}.", player, win_distance, move_string);
                                
                            },
                            Ordering::Equal => {
                                let player = if board.player_one { 1 } else { 2 };
                                println!("Player {} can at best force a draw, {} {} remaining", player, win_distance, move_string);
                            }
                        }

                        println!("Best move: {}", best_move + 1);
                        best_move + 1

                    // human player
                    } else {
                        print!("Move input > ");
                        stdout().flush().expect("Failed to flush to stdout!");
                        let mut input_str = String::new();
                        stdin.read_line(&mut input_str)?;
                        
                        match input_str.trim().parse::<usize>() {
                            Err(_) => {
                                println!("Invalid number: {}", input_str);
                                continue;
                            }
                            Ok(column) => column,
                        }
                    };

                if let Err(err) = board.play_checked(next_move) {
                    println!("{}", err);
                    // try the move again
                    continue;
                }
            }

            // end states
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
