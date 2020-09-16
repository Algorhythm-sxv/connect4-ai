use anyhow::{anyhow, Result};
use crossterm::{
    cursor::MoveTo,
    style::{style, Attribute, Color, PrintStyledContent},
    QueueableCommand,
};

use std::io::{stdout, Write};

use crate::{HEIGHT, WIDTH};
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Cell {
    PlayerOne,
    PlayerTwo,
    Empty,
}

impl Cell {
    fn is_empty(&self) -> bool {
        match self {
            Cell::Empty => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum GameState {
    Playing,
    PlayerOneWin,
    PlayerTwoWin,
    Draw,
}
#[derive(Clone)]
pub struct ArrayBoard {
    cells: [Cell; WIDTH * HEIGHT], // cells are stored left-to-right, bottom-to-top
    heights: [usize; WIDTH],
    pub player_one: bool,
    pub game: String,
    num_moves: usize,
    pub state: GameState,
}
impl ArrayBoard {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            cells: [Cell::Empty; WIDTH * HEIGHT],
            heights: [0; WIDTH],
            player_one: true,
            game: String::new(),
            num_moves: 0,
            state: GameState::Playing,
        }
    }

    #[allow(unused)]
    pub fn from_str(moves: &str) -> Result<Self> {
        let mut board = Self::new();

        for column_char in moves.chars() {
            match column_char.to_digit(10) {
                Some(column) => {
                    let _ = board.play_checked(column as usize)?;
                }
                _ => return Err(anyhow!("could not parse '{}' as a valid move", column_char)),
            }
        }
        Ok(board)
    }

    pub fn play_checked(&mut self, column_one_indexed: usize) -> Result<GameState> {
        if column_one_indexed < 1 || column_one_indexed > WIDTH {
            return Err(anyhow!(
                "Invalid move, column {} out of range. Columns must be between 1 and {}",
                column_one_indexed,
                WIDTH
            ));
        }
        let column = column_one_indexed - 1;
        if !self.playable(column) {
            return Err(anyhow!("Invalid move, column {} full", column_one_indexed));
        }

        if self.check_winning_move(column) {
            self.state = if self.player_one {
                GameState::PlayerOneWin
            } else {
                GameState::PlayerTwoWin
            }
        } else {
            self.state = if self.check_draw_move() {
                GameState::Draw
            } else {
                GameState::Playing
            };
        }
        self.play(column);
        self.game.push_str(&column_one_indexed.to_string());

        Ok(self.state)
    }

    pub fn check_draw_move(&self) -> bool {
        self.cells.iter().filter(|x| x.is_empty()).count() == 1
    }

    pub fn display(&self) -> Result<()> {
        let mut stdout = stdout();

        let cols: String = (1..=WIDTH).map(|x| x.to_string()).collect();
        stdout.queue(PrintStyledContent(style(cols + "\n")))?;
        for _ in 0..HEIGHT {
            stdout.queue(PrintStyledContent(style("\n")))?;
        }
        stdout.flush()?;

        let (origin_x, origin_y) = crossterm::cursor::position()?;

        for (idx, cell) in self.cells.iter().enumerate() {
            let (pos_x, pos_y) = (
                origin_x + (idx % WIDTH) as u16,
                origin_y - (idx / WIDTH) as u16,
            );

            stdout
                .queue(MoveTo(pos_x, pos_y))?
                .queue(PrintStyledContent(
                    style("O")
                        .attribute(Attribute::Bold)
                        .on(Color::DarkBlue)
                        .with(match cell {
                            Cell::PlayerOne => Color::Red,
                            Cell::PlayerTwo => Color::Yellow,
                            Cell::Empty => Color::DarkBlue,
                        }),
                ))?;
        }
        stdout
            .queue(MoveTo(origin_x + WIDTH as u16, origin_y))?
            .queue(PrintStyledContent(style("\n")))?;
        stdout.flush()?;
        Ok(())
    }
    fn playable(&self, column: usize) -> bool {
        self.heights[column] < HEIGHT
    }
    pub fn play(&mut self, column: usize) {
        let player = if self.player_one {
            Cell::PlayerOne
        } else {
            Cell::PlayerTwo
        };
        self.cells[column + WIDTH * self.heights[column]] = player;
        self.heights[column] += 1;
        self.num_moves += 1;
        self.player_one = !self.player_one;
    }
    fn check_winning_move(&self, column: usize) -> bool {
        let player = if self.player_one {
            Cell::PlayerOne
        } else {
            Cell::PlayerTwo
        };
        // check vertical alignment
        if self.heights[column] >= 3
            && self.cells[column + WIDTH * (self.heights[column] - 1)] == player
            && self.cells[column + WIDTH * (self.heights[column] - 2)] == player
            && self.cells[column + WIDTH * (self.heights[column] - 3)] == player
        {
            return true;
        }

        // check horizontal and diagonal alignment
        for dy_dx in -1i32..=1 {
            let mut run = 0;
            for dx in [-1i32, 1].iter() {
                let mut x = column as i32 + dx;
                let mut y = self.heights[column] as i32 + dx * dy_dx;
                loop {
                    if x < 0
                        || x >= WIDTH as i32
                        || y < 0
                        || y >= HEIGHT as i32
                        || self.cells[x as usize + WIDTH * y as usize] != player
                    {
                        break;
                    }
                    x += dx;
                    y += dx * dy_dx;
                    run += 1;
                }
            }
            if run >= 3 {
                return true;
            }
        }

        false
    }
}
