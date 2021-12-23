#[derive(Default, Clone, PartialEq, Eq)]
pub struct BitSet256 {
    // bit[3] bit[2] bit[1] bit[0]
    bit: [u64; 4],
}

impl BitSet256 {
    pub fn flip(&mut self, i: usize) {
        self.bit[i / 64] ^= 1 << (i % 64);
    }

    pub fn get(&self, i: usize) -> bool {
        (self.bit[i / 64] >> (i % 64) & 1) == 1
    }

    pub fn rot_left(&self, i: usize) -> Self {
        let mut b = self.clone();
        b.mut_rot_left(i);
        b
    }

    pub fn rot_right(&self, i: usize) -> Self {
        let mut b = self.clone();
        b.mut_rot_right(i);
        b
    }

    #[allow(clippy::needless_range_loop)]
    pub fn mut_rot_left(&mut self, i: usize) {
        let x = self.bit;

        let offset = i / 64;
        let rem = i % 64;
        if rem == 0 {
            for i in 0..4 {
                self.bit[(i + offset) % 4] = x[i];
            }
        } else {
            for i in 0..4 {
                self.bit[(i + offset) % 4] = x[i] << rem;
                self.bit[(i + offset) % 4] |= x[(i + 3) % 4] >> (64 - rem);
            }
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn mut_rot_right(&mut self, i: usize) {
        let x = self.bit;

        let offset = i / 64;
        let rem = i % 64;
        if rem == 0 {
            for i in 0..4 {
                self.bit[i] = x[(i + offset) % 4];
            }
        } else {
            for i in 0..4 {
                self.bit[i] = x[(i + offset) % 4] >> rem;
                self.bit[i] |= x[(i + offset + 1) % 4] << (64 - rem);
            }
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn to_vec(&self) -> Vec<bool> {
        let mut val = vec![false; 256];
        for i in 0..256 {
            val[i] = self.get(i);
        }
        val
    }
}

impl std::ops::BitOrAssign for BitSet256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bit
            .iter_mut()
            .zip(rhs.bit.iter())
            .for_each(|(lhs, rhs)| *lhs |= *rhs);
    }
}

impl std::ops::BitAnd for &BitSet256 {
    type Output = BitSet256;

    fn bitand(self, rhs: Self) -> BitSet256 {
        BitSet256 {
            bit: [
                self.bit[0] & rhs.bit[0],
                self.bit[1] & rhs.bit[1],
                self.bit[2] & rhs.bit[2],
                self.bit[3] & rhs.bit[3],
            ],
        }
    }
}

#[test]
fn flip() {
    let mut bitset = BitSet256::default();
    assert_eq!([0, 0, 0, 0], bitset.bit);

    bitset.flip(0);
    assert_eq!([1, 0, 0, 0], bitset.bit);

    bitset.flip(255);
    assert_eq!([1, 0, 0, 0x8000_0000_0000_0000], bitset.bit);
}

#[test]
fn rot_left() {
    let mut bitset = BitSet256::default();

    bitset.flip(0);
    assert_eq!([1 << 63, 0, 0, 0], bitset.rot_left(63).bit);
    assert_eq!([0, 1, 0, 0], bitset.rot_left(64).bit);
    assert_eq!([0, 0, 0, 0x8000_0000_0000_0000], bitset.rot_left(255).bit);
}

#[test]
fn rot_right() {
    let mut bitset = BitSet256::default();

    bitset.flip(0);
    assert_eq!([0, 0, 0, 1 << 63], bitset.rot_right(1).bit);
    assert_eq!([0, 0, 0, 1], bitset.rot_right(64).bit);
    assert_eq!([0, 0, 1 << 63, 0], bitset.rot_right(65).bit);
    assert_eq!([2, 0, 0, 0], bitset.rot_right(255).bit);
}

#[test]
fn overflow() {
    eprintln!(
        "{:064b}",
        0b1010101010101010101010101010101010101010101010101010101010101010_u64 << 32
    );
    eprintln!(
        "{:064b}",
        0b1010101010101010101010101010101010101010101010101010101010101010_u64 >> 32
    );
}
