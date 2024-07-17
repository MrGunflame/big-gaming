use std::cmp::Ordering;

pub use game_wasm::record::ModuleId;

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<Dependency>,
}

impl Module {
    // FIXME: Change this to constant if possible.
    pub fn core() -> Self {
        Self {
            id: ModuleId::CORE,
            name: String::from("core"),
            version: Version::new(1, 0, 0),
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dependency {
    pub id: ModuleId,
    pub name: Option<String>,
    //TODO
    pub version: Version,
}

pub trait ModuleIdExt {
    fn random() -> Self;
}

impl ModuleIdExt for ModuleId {
    fn random() -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self::from_bytes(uuid.into_bytes())
    }
}

/// A version of a [`Module`].
///
/// This is follows the rules for semver (<https://semver.org>), but without an additional
/// build-metadata section.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: PreRelease,
}

impl Version {
    /// A `Version` that has no other requirements as being used as a placeholder.
    ///
    /// The only guarantees given by this value is that it is equal to itself. No ordering
    /// guarantees are given and the internal components of this value should be considered
    /// opaque and may change over time.
    pub const PLACEHOLDER: Self = Self::new(0, 0, 0);

    /// Creates a new `Version` with the given `major`, `minor` and `patch` components.
    ///
    /// The `pre_release` is left empty.
    #[inline]
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: PreRelease::EMPTY,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.pre_release.cmp(&other.pre_release))
    }
}

/// A pre release tag of a [`Version`].
///
/// The default `PreRelease` tag is empty (equal to [`EMPTY`]).
///
/// [`EMPTY`]: Self::EMPTY
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PreRelease(String);

impl PreRelease {
    /// The empty `PreRelease` tag.
    pub const EMPTY: Self = Self(String::new());

    /// Creates a new `PreRelease` from the given `text`.
    #[inline]
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// Returns the underlying `str` that was used to create this `PreRelease`.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialOrd for PreRelease {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PreRelease {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut lhs = self.0.split('.');
        let mut rhs = other.0.split('.');

        loop {
            match (lhs.next(), rhs.next()) {
                (Some(lhs), Some(rhs)) => match lhs.cmp(rhs) {
                    Ordering::Less => return Ordering::Less,
                    Ordering::Greater => return Ordering::Greater,
                    Ordering::Equal => (),
                },
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => break,
            }
        }

        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::{PreRelease, Version};

    #[test]
    fn version_cmp() {
        let lhs = Version::new(0, 1, 0);
        let rhs = Version::new(0, 1, 1);
        assert!(lhs < rhs);

        let lhs = Version::new(0, 1, 1);
        let rhs = Version::new(1, 0, 0);
        assert!(lhs < rhs);
    }

    #[test]
    fn pre_release_cmp() {
        let lhs = PreRelease::new("alpha.12");
        let rhs = PreRelease::new("alpha.13");
        assert!(lhs < rhs);

        let lhs = PreRelease::new("alpha");
        let rhs = PreRelease::new("alpha.1");
        assert!(lhs < rhs);

        let lhs = PreRelease::new("alpha.1");
        let rhs = PreRelease::new("beta");
        assert!(lhs < rhs);
    }
}
