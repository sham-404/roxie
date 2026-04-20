pub struct Board {
    bitboards: [u64; 12],
}

impl Board {
    pub fn new() -> Self {
        let mut bitboards = [0u64; 12];

        // White pieces
        bitboards[Piece::BP as usize] = 0x00FF000000000000;
        bitboards[Piece::BN as usize] = 0x4200000000000000;
        bitboards[Piece::BB as usize] = 0x2400000000000000;
        bitboards[Piece::BR as usize] = 0x8100000000000000;
        bitboards[Piece::BQ as usize] = 0x0800000000000000;
        bitboards[Piece::BK as usize] = 0x1000000000000000;

        // Black pieces
        bitboards[Piece::WP as usize] = 0x000000000000FF00;
        bitboards[Piece::WN as usize] = 0x0000000000000042;
        bitboards[Piece::WB as usize] = 0x0000000000000024;
        bitboards[Piece::WR as usize] = 0x0000000000000081;
        bitboards[Piece::WQ as usize] = 0x0000000000000008;
        bitboards[Piece::WK as usize] = 0x0000000000000010;

        Self { bitboards }
    }

    pub fn print_board(&self) {
        for rank in (0..8).rev() {
            print!("{}  ", rank + 1);

            for file in 0..8 {
                let sq = rank * 8 + file;
                let mut found = false;

                for i in 0..12 {
                    if (self.bitboards[i] >> sq) & 1 == 1 {
                        let piece = Piece::from_val(i);
                        print!("{} ", Piece::piece_to_char(piece));
                        found = true;
                        break;
                    }
                }

                if !found {
                    print!(". ");
                }
            }

            println!();
        }

        println!("\n   a b c d e f g h\n");
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum Piece {
    WP = 0,
    WN = 1,
    WB = 2,
    WR = 3,
    WQ = 4,
    WK = 5,

    BP = 6,
    BN = 7,
    BB = 8,
    BR = 9,
    BQ = 10,
    BK = 11,
}

impl Piece {
    fn piece_to_char(piece: Piece) -> char {
        match piece {
            Piece::WP => 'P',
            Piece::WN => 'N',
            Piece::WB => 'B',
            Piece::WR => 'R',
            Piece::WQ => 'Q',
            Piece::WK => 'K',

            Piece::BP => 'p',
            Piece::BN => 'n',
            Piece::BB => 'b',
            Piece::BR => 'r',
            Piece::BQ => 'q',
            Piece::BK => 'k',
        }
    }

    fn from_val(val: usize) -> Self {
        let piece = match val {
            0 => Piece::WP,
            1 => Piece::WN,
            2 => Piece::WB,
            3 => Piece::WR,
            4 => Piece::WQ,
            5 => Piece::WK,

            6 => Piece::BP,
            7 => Piece::BN,
            8 => Piece::BB,
            9 => Piece::BR,
            10 => Piece::BQ,
            11 => Piece::BK,
            _ => unreachable!(),
        };

        piece
    }
}

#[repr(u8)]
enum MoveFlag {
    Quiet = 0b0000,
    DoublePush = 0b0001,
    KingCastle = 0b0010,
    QueenCastle = 0b0011,

    Capture = 0b1000,
    EnPassant = 0b1010,

    PromoKnight = 0b0100,
    PromoBishop = 0b0101,
    PromoRook = 0b0110,
    PromoQueen = 0b0111,

    PromoCapKnight = 0b1100,
    PromoCapBishop = 0b1101,
    PromoCapRook = 0b1110,
    PromoCapQueen = 0b1111,
}

struct Move {
    from: usize,
    to: usize,
    flag: MoveFlag,
}
