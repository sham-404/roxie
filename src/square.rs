#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Square(pub usize);

#[allow(dead_code)]
impl Square {
    pub fn new(index: usize) -> Self {
        Square(index)
    }

    pub fn index(self) -> usize {
        self.0
    }

    // Rank (0–7)
    pub fn rank(self) -> usize {
        self.0 / 8
    }

    // File (0–7)
    pub fn file(self) -> usize {
        self.0 % 8
    }

    // Create from rank + file
    pub fn from_rf(rank: usize, file: usize) -> Self {
        Square(rank * 8 + file)
    }

    // Convert to coordinate string
    pub fn to_coord(self) -> String {
        let file = (b'a' + self.file() as u8) as char;
        let rank = (b'1' + self.rank() as u8) as char;
        format!("{}{}", file, rank)
    }

    // Parse from coordinate string

    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() != 2 {
            return None;
        }

        let bytes = s.as_bytes();
        let file = bytes[0].wrapping_sub(b'a') as usize;
        let rank = bytes[1].wrapping_sub(b'1') as usize;

        if file < 8 && rank < 8 {
            Some(Square::from_rf(rank, file))
        } else {
            None
        }
    }

    pub fn offset(self, dr: i32, df: i32) -> Option<Square> {
        let r = self.rank() as i32 + dr;
        let f = self.file() as i32 + df;

        if (0..8).contains(&r) && (0..8).contains(&f) {
            Some(Square::from_rf(r as usize, f as usize))
        } else {
            None
        }
    }
}
