#![no_std]
use core::{
    cmp::{Eq, Ord, Ordering, PartialEq},
    mem::size_of,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use bytemuck::{Pod, Zeroable};

pub const NUMBER_SIZE: usize = 10 * u8::MAX as usize;
pub const BASE: u32 = u16::MAX as u32;
pub type BaseType = u32;
pub const BASE_SIZE: usize = size_of::<BaseType>();

type UpperBase = u32;
type SignedUpperBase = i64;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BigNumber {
    pub prec: [BaseType; NUMBER_SIZE],
}

unsafe impl Pod for BigNumber {}
unsafe impl Zeroable for BigNumber {}

fn collect_array<T, I, const N: usize>(itr: I) -> [T; N]
where
    T: Default + Copy,
    I: IntoIterator<Item = T>,
{
    let mut res = [T::default(); N];
    for (it, elem) in res.iter_mut().zip(itr) {
        *it = elem
    }

    res
}

impl Default for BigNumber {
    fn default() -> Self {
        Self::empty()
    }
}

impl BigNumber {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self {
            prec: [0; NUMBER_SIZE],
        }
    }

    pub fn from(num: BaseType) -> Self {
        let mut s = Self::new();
        s.prec[0] = num;
        s
    }

    pub fn from_upper(num: UpperBase) -> Self {
        let mut s = Self::new();

        s.prec[0] = num.rem_euclid(BASE) as BaseType;
        s.prec[1] = (num / BASE) as BaseType;
        s
    }

    pub fn is_zero(&self) -> bool {
        self.prec.iter().all(|&x| x == 0)
    }

    pub fn from_ne_bytes(bytes: &[u8; NUMBER_SIZE * BASE_SIZE]) -> Self {
        let prec = bytes
            .chunks(BASE_SIZE)
            .map(|b| BaseType::from_ne_bytes(b.try_into().unwrap()));

        Self {
            prec: collect_array::<BaseType, _, NUMBER_SIZE>(prec),
        }
    }

    pub fn leading_zeros(&self) -> usize {
        for &chunk in self.prec.iter().rev() {
            if chunk != 0 {
                return chunk.leading_zeros() as usize;
            }
        }
        NUMBER_SIZE * 32 // If the number is zero, return the size of the number
    }

    pub fn rotated_right(&mut self, shift: usize) {
        let shift = shift % NUMBER_SIZE;
        if shift != 0 {
            let mut temp = [0; NUMBER_SIZE];

            temp.copy_from_slice(&self.prec);

            for i in 0..NUMBER_SIZE {
                let j = (i + shift) % NUMBER_SIZE;
                self.prec[i] = temp[j];
            }
        }
    }
}

impl PartialEq for BigNumber {
    fn eq(&self, other: &Self) -> bool {
        self.prec.iter().zip(other.prec.iter()).all(|(a, b)| a == b)
    }
}

impl PartialOrd for BigNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for BigNumber {}

impl Ord for BigNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        for (a, b) in self.prec.iter().zip(other.prec.iter()).rev() {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }
}

impl AddAssign for BigNumber {
    fn add_assign(&mut self, rhs: Self) {
        let mut result = Self::new();
        let mut carry = 0;

        for i in 0..NUMBER_SIZE {
            let num = self.prec[i] as UpperBase + rhs.prec[i] as UpperBase + carry as UpperBase;

            result.prec[i] = (num % BASE) as BaseType;

            if num < BASE as UpperBase {
                carry = 0
            } else {
                carry = 1
            }
        }

        *self = result
    }
}

impl Add for BigNumber {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result += rhs;
        result
    }
}

impl SubAssign for BigNumber {
    fn sub_assign(&mut self, rhs: Self) {
        let mut result = Self::new();
        let mut carry = 0;

        if rhs > *self {
            *self = result;
            return;
        }

        for i in 0..NUMBER_SIZE {
            let num = self.prec[i] as SignedUpperBase - rhs.prec[i] as SignedUpperBase
                + carry as SignedUpperBase;

            result.prec[i] = num.rem_euclid(BASE as SignedUpperBase) as BaseType;

            if num >= 0 {
                carry = 0
            } else {
                carry = -1
            }
        }

        *self = result
    }
}

impl Sub for BigNumber {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result -= rhs;
        result
    }
}

impl MulAssign for BigNumber {
    fn mul_assign(&mut self, rhs: Self) {
        let mut w = Self::new().prec;
        let n = self.prec.len();
        let t = rhs.prec.len();

        for i in 0..t {
            let mut c = 0;
            for j in 0..n {
                if i + j > w.len() - 1 {
                    continue;
                }

                let uvb = w[i + j] as UpperBase
                    + self.prec[i] as UpperBase * rhs.prec[j] as UpperBase
                    + c as UpperBase;

                w[i + j] = uvb.rem_euclid(BASE as UpperBase) as BaseType;
                c = (uvb / BASE) as BaseType;
            }
        }

        self.prec = w
    }
}

impl Mul for BigNumber {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result *= rhs;
        result
    }
}

impl DivAssign for BigNumber {
    fn div_assign(&mut self, divisor: Self) {
        let mut quotient = BigNumber::new();
        let dividend = self;
        let mut remainder = *dividend;

        for i in (0..NUMBER_SIZE).rev() {
            let mut shifted_divisor = divisor;
            shifted_divisor.rotated_right(i);

            while remainder > shifted_divisor {
                remainder -= shifted_divisor;
                quotient.prec[i] += 1;
            }
        }

        let mut q_len = NUMBER_SIZE;
        while q_len > 1 && quotient.prec[q_len - 1] == 0 {
            q_len -= 1;
        }

        *dividend = quotient
    }
}

impl Div for BigNumber {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut result = self;
        result /= rhs;
        result
    }
}

// #[cfg(test)]
// mod test {
//     use crate::{BaseType, BigNumber};
//
//     #[test]
//     fn test_1() {
//         let mut data = BigNumber::from(u16::MAX as u32);
//
//         for _ in 0..10 {
//             data *= BigNumber::from((u16::MAX) as u32);
//         }
//
//         println!("number {:?}", data);
//     }
// }
