use std::{
    array,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use crossterm::cursor::MoveTo;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Vec2<T: Copy> {
    pub x: T,
    pub y: T,
}

#[allow(dead_code)]
impl<T: Copy> Vec2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub const fn splat(v: T) -> Self {
        Self::new(v, v)
    }

    pub fn with_x(self, x: T) -> Vec2<T> {
        Self { x, y: self.y }
    }

    pub fn with_y(self, y: T) -> Vec2<T> {
        Self { x: self.x, y }
    }

    pub fn zip<U: Copy>(self, other: Vec2<U>) -> Vec2<(T, U)> {
        Vec2 {
            x: (self.x, other.x),
            y: (self.y, other.y),
        }
    }

    pub fn map<U: Copy>(self, mut f: impl FnMut(T) -> U) -> Vec2<U> {
        Vec2 {
            x: f(self.x),
            y: f(self.y),
        }
    }

    pub fn map_x(mut self, f: impl FnOnce(T) -> T) -> Self {
        self.x = f(self.x);
        self
    }

    pub fn map_y(mut self, f: impl FnOnce(T) -> T) -> Self {
        self.y = f(self.y);
        self
    }

    pub fn join<U: Copy, O: Copy>(self, other: Vec2<U>, mut f: impl FnMut(T, U) -> O) -> Vec2<O> {
        self.zip(other).map(|(t, u)| f(t, u))
    }
}

impl<T: Copy + Mul<Output = T>> Vec2<T> {
    pub fn area(self) -> T {
        self.x * self.y
    }
}

impl<T: Copy + Into<usize>> Vec2<T> {
    pub fn index_row_major(self, width: usize) -> usize {
        self.y.into() * width + self.x.into()
    }
}

#[allow(dead_code)]
impl Vec2<u16> {
    pub const ZERO: Vec2<u16> = Vec2::new(0, 0);
    pub const ONE: Vec2<u16> = Vec2::new(1, 1);
    pub const X: Vec2<u16> = Vec2::new(1, 0);
    pub const Y: Vec2<u16> = Vec2::new(0, 1);

    #[must_use]
    pub fn move_to(self) -> MoveTo {
        MoveTo(self.x, self.y)
    }

    #[must_use]
    pub fn x(x: u16) -> Self {
        Self { x, y: 0 }
    }

    #[must_use]
    pub fn y(y: u16) -> Self {
        Self { x: 0, y }
    }
}

impl<T: Copy> IntoIterator for Vec2<T> {
    type Item = T;
    type IntoIter = array::IntoIter<T, 2>;

    fn into_iter(self) -> Self::IntoIter {
        [self.x, self.y].into_iter()
    }
}

impl<T: Copy> From<(T, T)> for Vec2<T> {
    fn from((x, y): (T, T)) -> Self {
        Vec2 { x, y }
    }
}

impl<T: Copy + Add<U, Output = O>, U: Copy, O: Copy> Add<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Add<U>>::Output>;

    fn add(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s + o)
    }
}

impl<T: Copy + Sub<U, Output = O>, U: Copy, O: Copy> Sub<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Sub<U>>::Output>;

    fn sub(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s - o)
    }
}

impl<T: Copy + Mul<U, Output = O>, U: Copy, O: Copy> Mul<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Mul<U>>::Output>;

    fn mul(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s * o)
    }
}

impl<T: Copy + Div<U, Output = O>, U: Copy, O: Copy> Div<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Div<U>>::Output>;

    fn div(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s / o)
    }
}

impl<T: Copy + AddAssign<U>, U: Copy> AddAssign<Vec2<U>> for Vec2<T> {
    fn add_assign(&mut self, rhs: Vec2<U>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Copy + SubAssign<U>, U: Copy> SubAssign<Vec2<U>> for Vec2<T> {
    fn sub_assign(&mut self, rhs: Vec2<U>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Copy + MulAssign<U>, U: Copy> MulAssign<Vec2<U>> for Vec2<T> {
    fn mul_assign(&mut self, rhs: Vec2<U>) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<T: Copy + DivAssign<U>, U: Copy> DivAssign<Vec2<U>> for Vec2<T> {
    fn div_assign(&mut self, rhs: Vec2<U>) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rect<T: Copy> {
    pub pos: Vec2<T>,
    pub size: Vec2<T>,
}

impl<T: Copy + AddAssign + SubAssign> Rect<T> {
    pub fn shrink_centered(mut self, factor: Vec2<T>) -> Rect<T> {
        self.pos += factor;
        self.size -= factor;
        self.size -= factor;
        self
    }
}
