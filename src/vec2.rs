use std::{
    array,
    ops::{Add, Div, Mul, Sub},
};

use crossterm::cursor::MoveTo;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

#[allow(dead_code)]
impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn zip<U>(self, other: Vec2<U>) -> Vec2<(T, U)> {
        Vec2 {
            x: (self.x, other.x),
            y: (self.y, other.y),
        }
    }

    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Vec2<U> {
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

    pub fn join<U, O>(self, other: Vec2<U>, mut f: impl FnMut(T, U) -> O) -> Vec2<O> {
        self.zip(other).map(|(t, u)| f(t, u))
    }
}

impl<T: Clone> Vec2<T> {
    pub fn splat(v: T) -> Self {
        Self::new(v.clone(), v)
    }
}

#[allow(dead_code)]
impl<T: Copy> Vec2<T> {
    pub fn with_x(self, x: T) -> Vec2<T> {
        Self { x, y: self.y }
    }

    pub fn with_y(self, y: T) -> Vec2<T> {
        Self { x: self.x, y }
    }
}

impl Vec2<u16> {
    #[must_use]
    pub fn move_to(self) -> MoveTo {
        MoveTo(self.x, self.y)
    }
}

impl<T> IntoIterator for Vec2<T> {
    type Item = T;
    type IntoIter = array::IntoIter<T, 2>;

    fn into_iter(self) -> Self::IntoIter {
        [self.x, self.y].into_iter()
    }
}

impl<T> From<(T, T)> for Vec2<T> {
    fn from((x, y): (T, T)) -> Self {
        Vec2 { x, y }
    }
}

impl<T: Add<U>, U> Add<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Add<U>>::Output>;

    fn add(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s + o)
    }
}

impl<T: Sub<U>, U> Sub<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Sub<U>>::Output>;

    fn sub(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s - o)
    }
}

impl<T: Mul<U>, U> Mul<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Mul<U>>::Output>;

    fn mul(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s * o)
    }
}

impl<T: Div<U>, U> Div<Vec2<U>> for Vec2<T> {
    type Output = Vec2<<T as Div<U>>::Output>;

    fn div(self, rhs: Vec2<U>) -> Self::Output {
        self.join(rhs, |s, o| s / o)
    }
}
