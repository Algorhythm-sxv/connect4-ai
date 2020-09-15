use anyhow::Result;

use std::fs::File;
use std::io::{BufReader, Read};
use std::rc::Rc;

const DATABASE_PATH: &str = "BookDeepDist.dat";
const DATABASE_NUM_POSITIONS: usize = 4200899;

#[derive(Clone)]
pub struct OpeningDatabase(Rc<OpeningDatabaseStorage>);

impl OpeningDatabase {
    pub fn load() -> Result<Self> {
        Ok(Self(Rc::new(OpeningDatabaseStorage::load()?)))
    }

    pub fn get(&self, position_code: u32, mirror_code: u32) -> i32 {
        self.0.get(position_code, mirror_code)
    }
}

#[derive(Clone)]
struct OpeningDatabaseStorage {
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

    pub fn get(&self, position_code: u32, mirror_code: u32) -> i32 {
        // variables for binary search state
        let mut step = DATABASE_NUM_POSITIONS - 1;
        let (mut pos1, mut pos2) = (step, step);

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
            let code2 = *self.positions.get(pos2).unwrap_or(&0);

            if position_code < code1 {
                // overflow is acceptable as the Vec::get earlier guards against panic
                pos1 = pos1.wrapping_sub(step);
            } else if position_code > code1 {
                // overflow is acceptable as the Vec::get earlier guards against panic
                pos1 = pos1.wrapping_add(step);
            } else {
                value = self.values[pos1];
                break;
            }

            if mirror_code < code2 {
                // overflow is acceptable as the Vec::get earlier guards against panic
                pos2 = pos2.wrapping_sub(step);
            } else if mirror_code > code2 {
                // overflow is acceptable as the Vec::get earlier guards against panic
                pos2 = pos2.wrapping_add(step);
            } else {
                value = self.values[pos2];
                break;
            }
        }
        // convert database value to local
        if value > 0 {
            let distance = 100 - value;
            return 21 - ((12 + distance) / 2) as i32;
        } else if value < 0 {
            let distance = 100 + value;
            return -22 + ((12 + distance) / 2) as i32;
        } else {
            return 0;
        }
    }
}
