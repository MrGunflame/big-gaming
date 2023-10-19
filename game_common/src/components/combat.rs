use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Display, Formatter};
use std::iter::FusedIterator;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use glam::Vec3;

use crate::id::WeakId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The health and maximum health values of an actor.
///
/// `Health` implements the [`Add`] and [`Sub`] operators which saturate at `max_health` and `0`
/// respectively.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Health {
    /// The current health value.
    pub health: u32,
    /// The maximum health value. This should never be higher than `health`.
    pub max_health: u32,
}

impl Health {
    /// Creates a new `Health` with given current health and maximum health values.
    #[inline]
    pub const fn new(health: u32) -> Self {
        Self {
            health,
            max_health: health,
        }
    }

    /// Returns `true` if the current health value is zero.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.health == 0
    }

    /// Returns `true` if the current health value equals the maximum health value.
    #[inline]
    pub const fn is_max(self) -> bool {
        self.health == self.max_health
    }
}

impl Add<u32> for Health {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        let mut health = self.health.saturating_add(rhs);
        if health > self.max_health {
            health = self.max_health;
        }

        Self {
            health,
            max_health: self.max_health,
        }
    }
}

impl AddAssign<u32> for Health {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl Sub<u32> for Health {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u32) -> Self::Output {
        let health = self.health.saturating_sub(rhs);

        Self {
            health,
            max_health: self.max_health,
        }
    }
}

impl SubAssign<u32> for Health {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

impl Display for Health {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.health, self.max_health)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageList {
    inner: Vec<Damage>,
}

impl DamageList {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn push(&mut self, damage: Damage) {
        self.inner.push(damage);
    }

    #[inline]
    pub fn iter(&self) -> DamageIter<'_> {
        DamageIter {
            inner: self,
            index: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DamageIter<'a> {
    inner: &'a DamageList,
    index: usize,
}

impl<'a> Iterator for DamageIter<'a> {
    type Item = Damage;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let elem = *self.inner.inner.get(self.index)?;
        self.index += 1;
        Some(elem)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for DamageIter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.inner.len() - self.index
    }
}

impl<'a> FusedIterator for DamageIter<'a> {}

/// A raw damage value.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feaure = "serde", derive(Serialize, Deserialize))]
pub struct Damage {
    pub class: DamageClass,
    pub amount: u32,
}

impl Damage {
    pub const fn new(amount: u32) -> Self {
        Self {
            class: DamageClass(WeakId(0)),
            amount,
        }
    }

    pub const fn with_class(mut self, class: DamageClass) -> Self {
        self.class = class;
        self
    }
}

/// A queue of incoming [`Damage`].
///
/// Every entity that should take damage should have a `IncomingDamage` component and call
/// [`push`] when damage should be taken instead of manually modifying the [`Health`] value.
///
/// [`push`]: Self::push
#[derive(Clone, Debug, Default)]
pub struct IncomingDamage {
    incoming: VecDeque<Damage>,
}

impl IncomingDamage {
    pub fn new() -> Self {
        Self {
            incoming: VecDeque::new(),
        }
    }

    pub fn clear(&mut self) {
        self.incoming.clear();
        self.incoming.shrink_to_fit();
    }

    /// Pushes a new [`Damage`] entry onto the queue.
    pub fn push(&mut self, damage: Damage) {
        self.incoming.push_back(damage);
    }

    /// Removes and retruns the oldest [`Damage`] entry from the queue. Returns `None` if the queue
    /// is empty.
    pub fn pop(&mut self) -> Option<Damage> {
        self.incoming.pop_front()
    }
}

impl Extend<Damage> for IncomingDamage {
    fn extend<T: IntoIterator<Item = Damage>>(&mut self, iter: T) {
        self.incoming.extend(iter);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Resistances {
    classes: HashMap<DamageClass, Resistance>,
}

impl Resistances {
    #[inline]
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }

    pub fn get<T>(&self, class: T) -> Option<Resistance>
    where
        T: Borrow<DamageClass>,
    {
        self.classes.get(class.borrow()).copied()
    }

    pub fn get_mut<T>(&mut self, class: T) -> Option<&mut Resistance>
    where
        T: Borrow<DamageClass>,
    {
        self.classes.get_mut(class.borrow())
    }

    pub fn add<T>(&mut self, class: T, value: Resistance) -> Resistance
    where
        T: Borrow<DamageClass>,
    {
        match self.get_mut(class.borrow()) {
            Some(res) => {
                *res += value;
                *res
            }
            None => {
                self.set(class, value);
                value
            }
        }
    }

    pub fn set<T>(&mut self, class: T, value: Resistance)
    where
        T: Borrow<DamageClass>,
    {
        self.classes.insert(*class.borrow(), value);
    }

    pub fn sub<T>(&mut self, class: T, value: Resistance) -> Option<Resistance>
    where
        T: Borrow<DamageClass>,
    {
        let res = self.get_mut(class)?;
        *res -= value;
        Some(*res)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.classes.iter(),
        }
    }
}

impl Add for Resistances {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl<'a> Add<&'a Self> for Resistances {
    type Output = Self;

    fn add(mut self, rhs: &'a Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Resistances {
    fn add_assign(&mut self, rhs: Self) {
        for (class, value) in &rhs {
            self.add(class, value);
        }
    }
}

impl<'a> AddAssign<&'a Self> for Resistances {
    fn add_assign(&mut self, rhs: &'a Self) {
        for (class, value) in rhs {
            self.add(class, value);
        }
    }
}

impl<'a> IntoIterator for &'a Resistances {
    type Item = (DamageClass, Resistance);
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    iter: std::collections::hash_map::Iter<'a, DamageClass, Resistance>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (DamageClass, Resistance);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(a, b)| (*a, *b))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a> FusedIterator for Iter<'a> {}

/// A damage "type".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct DamageClass(pub WeakId<u32>);

impl DamageClass {
    pub const BALLISTIC: Self = Self(WeakId(2));
    pub const ENERGY: Self = Self(WeakId(3));
}

/// A resistance value.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Resistance(pub u32);

impl Resistance {
    #[inline]
    pub const fn new(val: u32) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

impl Add for Resistance {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Resistance {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Resistance {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Resistance {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// An attack event.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Attack {
    /// The target (point) that the attack is targeted at.
    pub target: Vec3,
}

/// A reload event.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reload;