//! Contains types relating to operating system signals

use std::fmt;
use std::iter::FromIterator;
use std::ops;

/// Signal received through a terminal device
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Signal {
    /// Break signal (`CTRL_BREAK_EVENT`); Windows only
    Break,
    /// Continue signal (`SIGCONT`); Unix only
    Continue,
    /// Interrupt signal (`SIGINT` on Unix, `CTRL_C_EVENT` on Windows)
    Interrupt,
    /// Terminal window resize (`SIGWINCH` on Unix,
    /// `WINDOW_BUFFER_SIZE_EVENT` on Windows)
    ///
    /// When this signal is received, it will be translated into an
    /// `Event::Resize(_)` value containing the new size of the terminal.
    Resize,
    /// Suspend signal (`SIGTSTP`); Unix only
    Suspend,
    /// Quit signal (`SIGQUIT`); Unix only
    Quit,
}

const NUM_SIGNALS: u8 = 6;

impl Signal {
    fn as_bit(&self) -> u8 {
        1 << (*self as u8)
    }

    fn all_bits() -> u8 {
        (1 << NUM_SIGNALS) - 1
    }
}

impl ops::BitOr for Signal {
    type Output = SignalSet;

    fn bitor(self, rhs: Signal) -> SignalSet {
        let mut set = SignalSet::new();

        set.insert(self);
        set.insert(rhs);
        set
    }
}

impl ops::Not for Signal {
    type Output = SignalSet;

    fn not(self) -> SignalSet {
        !SignalSet::from(self)
    }
}

/// Represents a set of `Signal` values
#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub struct SignalSet(u8);

impl SignalSet {
    /// Returns an empty `SignalSet`.
    pub fn new() -> SignalSet {
        SignalSet(0)
    }

    /// Returns a `SignalSet` containing all available signals.
    pub fn all() -> SignalSet {
        SignalSet(Signal::all_bits())
    }

    /// Returns whether this set contains the given `Signal`.
    pub fn contains(&self, sig: Signal) -> bool {
        self.0 & sig.as_bit() != 0
    }

    /// Returns whether this set contains all signals present in another set.
    pub fn contains_all(&self, other: SignalSet) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether this set contains any signals present in another set.
    pub fn intersects(&self, other: SignalSet) -> bool {
        self.0 & other.0 != 0
    }

    /// Returns whether this set contains any signals.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Inserts a `Signal` into this set.
    pub fn insert(&mut self, sig: Signal) {
        self.0 |= sig.as_bit();
    }

    /// Removes a `Signal` from this set.
    pub fn remove(&mut self, sig: Signal) {
        self.0 &= !sig.as_bit();
    }

    /// Sets whether this set contains the given `Signal`.
    pub fn set(&mut self, sig: Signal, set: bool) {
        if set {
            self.insert(sig);
        } else {
            self.remove(sig);
        }
    }

    /// Returns the difference of two sets.
    ///
    /// The result is all signals contained in `self`, except for those
    /// also contained in `other`.
    ///
    /// This is equivalent to `self - other` or `self & !other`.
    pub fn difference(&self, other: SignalSet) -> SignalSet {
        SignalSet(self.0 & !other.0)
    }

    /// Returns the symmetric difference of two sets.
    ///
    /// The result is all signals contained in either set, but not those contained
    /// in both.
    ///
    /// This is equivalent to `self ^ other`.
    pub fn symmetric_difference(&self, other: SignalSet) -> SignalSet {
        SignalSet(self.0 ^ other.0)
    }

    /// Returns the intersection of two sets.
    ///
    /// The result is all signals contained in both sets, but not those contained
    /// in either one or the other.
    ///
    /// This is equivalent to `self & other`.
    pub fn intersection(&self, other: SignalSet) -> SignalSet {
        SignalSet(self.0 & other.0)
    }

    /// Returns the union of two sets.
    ///
    /// The result is all signals contained in either or both sets.
    ///
    /// This is equivalent to `self | other`.
    pub fn union(&self, other: SignalSet) -> SignalSet {
        SignalSet(self.0 | other.0)
    }

    /// Returns the inverse of the set.
    ///
    /// The result is all valid signals not contained in this set.
    ///
    /// This is equivalent to `!self`.
    pub fn inverse(&self) -> SignalSet {
        SignalSet(!self.0 & Signal::all_bits())
    }
}

impl fmt::Debug for SignalSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const SIGNALS: &[Signal] = &[
            Signal::Break,
            Signal::Continue,
            Signal::Interrupt,
            Signal::Resize,
            Signal::Suspend,
            Signal::Quit,
        ];

        let mut first = true;

        f.write_str("SignalSet(")?;

        for &sig in SIGNALS {
            if self.contains(sig) {
                if !first {
                    f.write_str(" | ")?;
                }

                write!(f, "{:?}", sig)?;
                first = false;
            }
        }

        f.write_str(")")
    }
}

impl From<Signal> for SignalSet {
    fn from(sig: Signal) -> SignalSet {
        let mut set = SignalSet::new();
        set.insert(sig);
        set
    }
}

impl Extend<Signal> for SignalSet {
    fn extend<I: IntoIterator<Item=Signal>>(&mut self, iter: I) {
        for sig in iter {
            self.insert(sig);
        }
    }
}

impl FromIterator<Signal> for SignalSet {
    fn from_iter<I: IntoIterator<Item=Signal>>(iter: I) -> SignalSet {
        let mut set = SignalSet::new();

        set.extend(iter);
        set
    }
}

macro_rules! impl_op {
    ( $tr:ident , $tr_meth:ident , $method:ident ) => {
        impl ops::$tr for SignalSet {
            type Output = SignalSet;

            fn $tr_meth(self, rhs: SignalSet) -> SignalSet {
                self.$method(rhs)
            }
        }
    }
}

macro_rules! impl_mut_op {
    ( $tr:ident , $tr_meth:ident , $method:ident ) => {
        impl ops::$tr for SignalSet {
            fn $tr_meth(&mut self, rhs: SignalSet) {
                *self = self.$method(rhs);
            }
        }
    }
}

macro_rules! impl_unary_op {
    ( $tr:ident , $tr_meth:ident , $method:ident ) => {
        impl ops::$tr for SignalSet {
            type Output = SignalSet;

            fn $tr_meth(self) -> SignalSet {
                self.$method()
            }
        }
    }
}

impl_op!{ BitAnd, bitand, intersection }
impl_op!{ BitOr, bitor, union }
impl_op!{ BitXor, bitxor, symmetric_difference }
impl_op!{ Sub, sub, difference }

impl_unary_op!{ Not, not, inverse }

impl_mut_op!{ BitAndAssign, bitand_assign, intersection }
impl_mut_op!{ BitOrAssign, bitor_assign, union }
impl_mut_op!{ BitXorAssign, bitxor_assign, symmetric_difference }
impl_mut_op!{ SubAssign, sub_assign, difference }

#[cfg(test)]
mod test {
    use super::{Signal, SignalSet};

    #[test]
    fn test_signal_set() {
        let mut set = SignalSet::new();

        assert!(set.is_empty());

        set.insert(Signal::Break);

        assert!(set.contains(Signal::Break));

        set |= Signal::Continue | Signal::Interrupt;

        assert!(set.contains(Signal::Break));
        assert!(set.contains(Signal::Continue));
        assert!(set.contains(Signal::Interrupt));

        set ^= Signal::Interrupt | Signal::Quit;

        assert!(set.contains(Signal::Break));
        assert!(!set.contains(Signal::Interrupt));
        assert!(set.contains(Signal::Quit));

        set &= Signal::Break | Signal::Suspend;

        assert!(set.contains(Signal::Break));
        assert!(!set.contains(Signal::Continue));
        assert!(!set.contains(Signal::Interrupt));
        assert!(!set.contains(Signal::Suspend));
        assert!(!set.contains(Signal::Quit));

        set -= SignalSet::from(Signal::Break);

        assert!(!set.contains(Signal::Break));
    }

    #[test]
    fn test_signal_set_all() {
        let mut all = SignalSet::all();

        assert!(all.contains(Signal::Break));
        assert!(all.contains(Signal::Continue));
        assert!(all.contains(Signal::Interrupt));
        assert!(all.contains(Signal::Resize));
        assert!(all.contains(Signal::Suspend));
        assert!(all.contains(Signal::Quit));

        assert_eq!(all, !SignalSet::new());
        assert_eq!(!all, SignalSet::new());

        all.remove(Signal::Break);
        all.remove(Signal::Continue);
        all.remove(Signal::Interrupt);
        all.remove(Signal::Resize);
        all.remove(Signal::Suspend);
        all.remove(Signal::Quit);

        assert_eq!(all.0, 0);
    }

    #[test]
    fn test_signal_set_debug() {
        let mut set = SignalSet::new();

        assert_eq!(format!("{:?}", set), "SignalSet()");

        set.insert(Signal::Break);
        assert_eq!(format!("{:?}", set), "SignalSet(Break)");

        set.insert(Signal::Continue);
        assert_eq!(format!("{:?}", set), "SignalSet(Break | Continue)");

        set.insert(Signal::Interrupt);
        assert_eq!(format!("{:?}", set), "SignalSet(Break | Continue | Interrupt)");

        set = SignalSet::all();
        assert_eq!(format!("{:?}", set),
            "SignalSet(Break | Continue | Interrupt | Resize | Suspend | Quit)");
    }
}
