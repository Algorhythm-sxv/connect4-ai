#[cfg(test)]
pub mod test {
    use anyhow::{anyhow, Result};
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::time::{Duration, Instant};

    use crate::{BitBoard, OpeningDatabase, Solver};

    #[test]
    pub fn huffman_coding() -> Result<()> {
        let board = BitBoard::from_str("22244444")?;
        let code = board.huffman_code();

        assert_eq!(code, 0b010111000111011101100000);
        Ok(())
    }
    #[test]
    pub fn opening_database() -> Result<()> {
        let openings = OpeningDatabase::load()?;

        // 0x81573efc
        //      Y
        //     RY
        //     YY
        //     RR
        //     RY
        //R____RY >>> 676766776717
        // 0x055daefc
        //     YY
        //     RY
        //     YY
        //     RR
        //     RY
        //_____RR >>> 777767676666
        //
        //
        //   Y
        //   R
        //   Y
        //Y  R
        //RRYYYRR >> 112364444475
        let mut solver = Solver::new(BitBoard::from_str("676766776717")?);
        let (calc, _) = solver.solve();

        let score = openings.get(
            solver.board.huffman_code(),
            solver.board.huffman_code_mirror(),
        );
        assert_eq!(score, calc);

        solver = Solver::new(BitBoard::from_str("777767676666")?);
        let (calc, _) = solver.solve();

        let score = openings.get(
            solver.board.huffman_code(),
            solver.board.huffman_code_mirror(),
        );

        assert_eq!(calc, score);

        solver = Solver::new(BitBoard::from_str("112364444475")?);
        let (calc, _) = solver.solve();

        let score = openings.get(
            solver.board.huffman_code(),
            solver.board.huffman_code_mirror(),
        );

        assert_eq!(calc, score);

        Ok(())
    }

    #[test]
    pub fn end_easy() -> Result<()> {
        let file = BufReader::new(File::open("test_data/Test_L3_R1")?);

        let mut times = vec![];
        let mut posis = vec![];

        for line in file.split(b'\n') {
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
            let (calc, _) = solver.solve();
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

        for line in file.split(b'\n') {
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
            let (calc, _) = solver.solve();
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

        for line in file.split(b'\n') {
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
            let (calc, _best) = solver.solve();
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

        for line in file.split(b'\n') {
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
            let mut solver = Solver::new(board).with_opening_database(OpeningDatabase::load()?);
            let start_time = Instant::now();
            let (calc, _best) = solver.solve();
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

    #[test]
    pub fn full_search() -> Result<()> {
        let board = BitBoard::new();
        let mut solver = Solver::new(board).with_opening_database(OpeningDatabase::load()?);
        let start_time = Instant::now();
        let (calc, best) = solver.solve();
        let finish_time = Instant::now();
        let time = finish_time - start_time;
        let posis = solver.node_count;

        println!(
            "Full game search\n Time: {:.6}s, No. of positions: {}, kpos/s: {}",
            time.as_secs_f64(),
            posis,
            posis as f64 / (1000.0 * time.as_secs_f64())
        );
        println!("Calculated score: {}, Best move: {}", calc, best + 1);
        Ok(())
    }
}
