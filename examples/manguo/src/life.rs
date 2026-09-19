//! Conway's Game of Life on the 16x16 sequencer grid.
//!
//! Cells are stored column-major to match `SequencerSettings.steps`:
//! `cells[x]` bit `y` is the cell in step column `x`, note row `y`.

pub const W: usize = 16;
pub const H: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Life {
    pub cells: [u16; W],
}

#[derive(Clone, Copy, Debug)]
pub enum Pattern {
    Blinker,
    Glider,
    Lwss,
    RPentomino,
    Blob,
}

impl Pattern {
    pub fn pick(seed: &mut u32) -> Pattern {
        match next_u32(seed) % 6 {
            0 => Pattern::Blinker,
            1 | 2 => Pattern::Glider,
            3 => Pattern::Lwss,
            4 => Pattern::RPentomino,
            _ => Pattern::Blob,
        }
    }
}

/// Cheap xorshift for deterministic, allocation-free randomness on the UI thread.
pub fn next_u32(seed: &mut u32) -> u32 {
    let mut x = *seed;
    if x == 0 {
        x = 0x9E37_79B9;
    }
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *seed = x;
    x
}

impl Life {
    pub fn from_steps(steps: &[u32]) -> Life {
        let mut cells = [0u16; W];
        for (x, s) in steps.iter().take(W).enumerate() {
            cells[x] = (*s & 0xFFFF) as u16;
        }
        Life { cells }
    }

    pub fn to_steps(&self) -> Vec<u32> {
        self.cells.iter().map(|c| *c as u32).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| *c == 0)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.cells[x % W] & (1 << (y % H)) != 0
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, on: bool) {
        let bit = 1u16 << (y % H);
        if on {
            self.cells[x % W] |= bit;
        } else {
            self.cells[x % W] &= !bit;
        }
    }

    /// One generation of B3/S23 on a torus.
    pub fn step(&self) -> Life {
        let mut next = Life::default();
        for x in 0..W {
            let xl = (x + W - 1) % W;
            let xr = (x + 1) % W;
            for y in 0..H {
                let yu = (y + H - 1) % H;
                let yd = (y + 1) % H;
                let n = self.get(xl, yu) as u8
                    + self.get(x, yu) as u8
                    + self.get(xr, yu) as u8
                    + self.get(xl, y) as u8
                    + self.get(xr, y) as u8
                    + self.get(xl, yd) as u8
                    + self.get(x, yd) as u8
                    + self.get(xr, yd) as u8;
                let alive = self.get(x, y);
                next.set(x, y, matches!((alive, n), (true, 2) | (true, 3) | (false, 3)));
            }
        }
        next
    }

    /// Stamp a pattern with its top-left at (x, y), wrapping on the torus.
    pub fn stamp(&mut self, pattern: Pattern, x: usize, y: usize, seed: &mut u32) {
        let shape: &[(usize, usize)] = match pattern {
            Pattern::Blinker => &[(0, 0), (1, 0), (2, 0)],
            Pattern::Glider => &[(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)],
            Pattern::Lwss => &[(1, 0), (4, 0), (0, 1), (0, 2), (4, 2), (0, 3), (1, 3), (2, 3), (3, 3)],
            Pattern::RPentomino => &[(1, 0), (2, 0), (0, 1), (1, 1), (1, 2)],
            Pattern::Blob => {
                for dy in 0..3 {
                    for dx in 0..3 {
                        if next_u32(seed) & 1 == 1 {
                            self.set(x + dx, y + dy, true);
                        }
                    }
                }
                return;
            }
        };
        for (dx, dy) in shape {
            self.set(x + dx, y + dy, true);
        }
    }
}
