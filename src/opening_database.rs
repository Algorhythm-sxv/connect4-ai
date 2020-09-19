use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indicatif::*;
use rayon::prelude::*;

use std::cmp::Ordering;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read};
use std::rc::Rc;
use std::sync::mpsc::*;
use std::thread;
use std::time::*;

use crate::*;

pub const DATABASE_PATH: &str = "opening_database.bin";
pub const TEMP_FILE_PATH: &str = "temp_positions.bin";
pub const DATABASE_DEPTH: usize = 12;
pub const DATABASE_NUM_POSITIONS: usize = 4200899;

#[derive(Clone)]
pub struct OpeningDatabase(Rc<OpeningDatabaseStorage>);

impl OpeningDatabase {
    pub fn load() -> Result<Self> {
        Ok(Self(Rc::new(OpeningDatabaseStorage::load()?)))
    }

    pub fn generate() -> Result<()> {
        let start = Instant::now();
        let mut next_time = start;

        let mut positions = Vec::new();

        // try to read positions from temp file
        if std::path::Path::new(TEMP_FILE_PATH).exists() {
            println!("Loading stored positions from {}", TEMP_FILE_PATH);
            let mut positions_file = BufReader::new(File::open(TEMP_FILE_PATH)?);
            for _ in 0..DATABASE_NUM_POSITIONS {
                positions.push((
                    positions_file.read_u32::<BigEndian>()?,
                    positions_file.read_u64::<BigEndian>()?,
                    positions_file.read_u64::<BigEndian>()?,
                ));
            }
        } else {
            enum Message {
                Count(usize),
                // remaining positions generated, Vec<huffman code, player mask, board mask>
                Finish((usize, Vec<(u32, u64, u64)>)),
            }
            let (tx, rx) = channel();

            for i in 0..WIDTH {
                let tx = tx.clone();

                thread::spawn(move || {
                    let mut moves = [0; DATABASE_DEPTH];
                    moves[0] = i;
                    let mut positions = Vec::new();
                    let mut generated = 0usize;
                    let mut last_size = 0;
                    let mut next_time = start + Duration::from_millis(100);

                    loop {
                        let mut iter = moves.iter().skip(1).take(HEIGHT + 1);
                        if iter.all(|&x| x == WIDTH - 1) {
                            tx.send(Message::Finish((generated, positions))).unwrap();
                            break;
                        }

                        if let Ok(board) = BitBoard::from_slice(&moves) {
                            // don't include next-turn wins, the tree search short-circuits these
                            // before searching the database
                            if !move_order()
                                .iter()
                                .any(|&i| board.playable(i) && board.check_winning_move(i))
                            {
                                // both mirrors will push the same huffman code, we will dedup later
                                positions.push((
                                    board.huffman_code().min(board.huffman_code_mirror()),
                                    board.player_mask(),
                                    board.board_mask(),
                                ));
                                generated += 1;
                            }
                        }

                        moves[DATABASE_DEPTH - 1] += 1;
                        // carry the addition
                        for d in (0..DATABASE_DEPTH).rev() {
                            if moves[d] >= WIDTH {
                                moves[d] = 0;
                                // d-1 should never underflow since the loop ends before that point is reached
                                moves[d - 1] += 1;
                            }
                        }
                        if Instant::now() > next_time {
                            if positions.len() - last_size > 10_000_000 {
                                positions.sort_unstable();
                                positions.dedup_by(|a, b| a.0 == b.0);
                                last_size = positions.len();
                            }
                            tx.send(Message::Count(generated)).unwrap();
                            generated = 0;
                            next_time += Duration::from_millis(500);
                        }
                    }
                });
            }

            let progress = ProgressBar::new(8532690438);
            progress.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[1/2] Generating positions: {bar:40.cyan/blue} {msg} ~{eta} remaining",
                    )
                    .progress_chars("█▓▒░  "),
            );

            let mut generated = 0usize;

            let mut finished = 0;
            while finished < WIDTH {
                match rx.recv()? {
                    Message::Count(num) => generated += num,
                    Message::Finish((thread_generated, mut thread_positions)) => {
                        generated += thread_generated;
                        positions.append(&mut thread_positions);
                        positions.sort_unstable();
                        positions.dedup_by(|a, b| a.0 == b.0);

                        finished += 1;
                    }
                }
                if Instant::now() > next_time {
                    progress.set_position(generated as u64);
                    progress.set_message(&format!(
                        "({}M / {}M)",
                        progress.position() / 1_000_000,
                        progress.length() / 1_000_000
                    ));
                    next_time += Duration::from_millis(100);
                }
            }

            let finish = Instant::now();
            progress.finish();
            println!(
                "Position generation complete in {:.1}s, found {} unique positions",
                (finish - start).as_secs_f64(),
                positions.len(),
            );
            print!("Writing out positions to {} ... ", TEMP_FILE_PATH);

            let mut positions_file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(TEMP_FILE_PATH)?;

            for position in positions.iter() {
                positions_file.write_u32::<BigEndian>(position.0)?;
                positions_file.write_u64::<BigEndian>(position.1)?;
                positions_file.write_u64::<BigEndian>(position.2)?;
            }

            println!("Complete");
        }

        enum Message2 {
            Value((u32, i8)),
            Finish,
        }
        let (tx, rx) = channel();

        let progress = ProgressBar::new(positions.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("[2/2] Calculating scores: {bar:40.cyan/blue} {msg} ~{eta} remaining")
                .progress_chars("█▓▒░  "),
        );

        let mut running = true;
        thread::spawn(move || {
            positions.par_iter().for_each_with(
                tx.clone(),
                |tx, (huffman_code, player_mask, board_mask)| {
                    let board = BitBoard::from_masks(*player_mask, *board_mask, 12);

                    let mut solver = Solver::new(board);
                    let (score, _) = solver.solve();

                    tx.send(Message2::Value((*huffman_code, score as i8)))
                        .unwrap();
                },
            );
            tx.send(Message2::Finish).unwrap();
        });

        let mut entries = Vec::new();
        let mut delta = 0;
        while running {
            match rx.recv()? {
                Message2::Finish => running = false,
                Message2::Value(entry) => {
                    entries.push(entry);
                    delta += 1;
                }
            }
            if Instant::now() > next_time {
                progress.inc(delta);
                delta = 0;
                progress.set_message(&format!(
                    "({} / {})",
                    progress.position(),
                    progress.length()
                ));
                next_time += Duration::from_millis(100);
            }
        }

        progress.finish();

        print!(
            "Calculations complete, writing out to {} ... ",
            DATABASE_PATH
        );

        entries.sort_unstable();

        let mut file = BufWriter::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(DATABASE_PATH)?,
        );

        for entry in entries {
            file.write_u32::<BigEndian>(entry.0)?;
            file.write_i8(entry.1)?;
        }
        println!("Complete");

        let finish = Instant::now();
        println!(
            "Opening database generation completed in {}",
            HumanDuration(finish - start)
        );

        Ok(())
    }
}

#[derive(Clone)]
pub struct OpeningDatabaseStorage {
    positions: Vec<u32>,
    values: Vec<i8>,
}

impl OpeningDatabaseStorage {
    pub fn load() -> Result<Self> {
        let mut file = BufReader::new(File::open(DATABASE_PATH)?);
        let mut positions = vec![0; DATABASE_NUM_POSITIONS];
        let mut values = vec![0; DATABASE_NUM_POSITIONS];

        for i in 0..DATABASE_NUM_POSITIONS {
            // read encoded position and winner
            let mut bytes = [0; 4];
            file.read_exact(&mut bytes)?;

            positions[i] = u32::from_be_bytes(bytes);

            let mut byte = [0];
            file.read_exact(&mut byte)?;
            values[i] = i8::from_be_bytes(byte);
        }
        Ok(Self { positions, values })
    }

    pub fn get(&self, position_code: u32) -> i32 {
        // variables for binary search state
        let mut step = DATABASE_NUM_POSITIONS - 1;
        let mut pos1 = step;

        // invalid value
        let mut value = -1;

        // Binary search
        while step > 0 {
            // divide step by 2, always rounding up apart from at 0.5
            step = if step != 1 {
                (step + (step & 1)) >> 1
            } else {
                0
            };

            // only one of the position code and its mirror will be present,
            // so one of these indices can become invalid
            let code1 = *self.positions.get(pos1).unwrap_or(&0);

            match position_code.cmp(&code1) {
                // overflow is acceptable as the Vec::get earlier guards against panic
                Ordering::Less => pos1 = pos1.wrapping_sub(step),
                Ordering::Greater => pos1 = pos1.wrapping_add(step),
                Ordering::Equal => {
                    value = self.values[pos1];
                    break;
                }
            }
        }
        value as i32
    }
}

impl std::ops::Deref for OpeningDatabase {
    type Target = OpeningDatabaseStorage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
