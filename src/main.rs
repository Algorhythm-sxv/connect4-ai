use connect4_ai::*;

use std::io::{stdin, stdout, Write};

fn main() {
    let mut board = ArrayBoard::new();
    println!("Welcome to Connect 4\n");

    let stdin = stdin();
    loop {
        board.display().expect("Failed to draw board!");

        print!("> ");
        stdout().flush().expect("Failed to flush to stdout!");
        let mut input_str = String::new();
        stdin
            .read_line(&mut input_str)
            .expect("Failed to read stdin!");

        let input = match input_str.trim().parse::<usize>() {
            Err(_) => {
                println!("Invalid number: {}", input_str);
                continue;
            }
            Ok(column) => column,
        };

        match board.play_checked(input) {
            Err(err) => {
                println!("{}", err);
                continue;
            }
            Ok(state) => match state {
                GameState::Playing => {}
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
            },
        }
    }
}