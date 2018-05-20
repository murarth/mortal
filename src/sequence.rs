//! Utilities for manipulating raw input sequences

use std::fmt;
use std::iter::FromIterator;
use std::mem::replace;

/// Contains a set of string sequences, mapped to a value.
#[derive(Clone, Debug, Default)]
pub struct SequenceMap<K, V> {
    sequences: Vec<(K, V)>,
}

/// Represents the result of a `SequenceMap::find` operation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FindResult<V> {
    /// No contained sequences begin with the provided input sequence.
    NotFound,
    /// One or more sequences begin with the provided input sequence,
    /// but the sequence does not represent a complete sequence.
    Incomplete,
    /// A sequence was found exactly matching the input sequence;
    /// additionally, one or more sequences begin with the input sequence.
    Undecided(V),
    /// A sequence was found exactly matching the input sequence;
    /// no additional partially-matching sequences exist.
    Found(V),
}

impl<'a, V: Clone> FindResult<&'a V> {
    /// Maps `FindResult<&V>` to `FindResult<V>` by cloning the contents
    /// of the result value.
    pub fn cloned(self) -> FindResult<V> {
        match self {
            FindResult::NotFound => FindResult::NotFound,
            FindResult::Incomplete => FindResult::Incomplete,
            FindResult::Undecided(v) => FindResult::Undecided(v.clone()),
            FindResult::Found(v) => FindResult::Found(v.clone()),
        }
    }
}

impl<K: AsRef<str>, V> SequenceMap<K, V> {
    /// Creates an empty `SequenceMap`.
    pub fn new() -> SequenceMap<K, V> {
        SequenceMap::with_capacity(0)
    }

    /// Creates an empty `SequenceMap` with allocated capacity for `n` elements.
    pub fn with_capacity(n: usize) -> SequenceMap<K, V> {
        SequenceMap{
            sequences: Vec::with_capacity(n),
        }
    }

    /// Returns a slice of all contained sequences, sorted by key.
    pub fn sequences(&self) -> &[(K, V)] {
        &self.sequences
    }

    /// Returns a mutable slice of all contained sequences, sorted by key.
    ///
    /// # Note
    ///
    /// Elements must remain sorted by key for the proper functioning of
    /// `SequenceMap` operations. If keys are modified, the caller must ensure
    /// that the slice is sorted.
    pub fn sequences_mut(&mut self) -> &mut [(K, V)] {
        &mut self.sequences
    }

    /// Returns an `Entry` for the given key.
    ///
    /// This API matches the entry API for the standard `HashMap` collection.
    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        match self.search(key.as_ref()) {
            Ok(n) => Entry::Occupied(OccupiedEntry{
                map: self,
                index: n,
            }),
            Err(n) => Entry::Vacant(VacantEntry{
                map: self,
                key,
                index: n,
            })
        }
    }

    /// Performs a search for a partial or complete sequence match.
    pub fn find(&self, key: &str) -> FindResult<&V> {
        let (n, found) = match self.search(key) {
            Ok(n) => (n, true),
            Err(n) => (n, false)
        };

        let incomplete = self.sequences.get(n + (found as usize))
            .map_or(false, |&(ref next, _)| next.as_ref().starts_with(key));

        match (found, incomplete) {
            (false, false) => FindResult::NotFound,
            (false, true) => FindResult::Incomplete,
            (true, false) => FindResult::Found(&self.sequences[n].1),
            (true, true) => FindResult::Undecided(&self.sequences[n].1),
        }
    }

    /// Returns the corresponding value for the given sequence.
    pub fn get(&self, key: &str) -> Option<&V> {
        match self.search(key) {
            Ok(n) => Some(&self.sequences[n].1),
            Err(_) => None
        }
    }

    /// Returns a mutable reference to the corresponding value for the given sequence.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        match self.search(key) {
            Ok(n) => Some(&mut self.sequences[n].1),
            Err(_) => None
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the key already exists in the map, the new value will replace the old
    /// value and the old value will be returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(key.as_ref()) {
            Ok(n) => Some(replace(&mut self.sequences[n], (key, value)).1),
            Err(n) => {
                self.sequences.insert(n, (key, value));
                None
            }
        }
    }

    /// Removes a key-value pair from the map.
    pub fn remove(&mut self, key: &str) -> Option<(K, V)> {
        match self.search(key) {
            Ok(n) => Some(self.sequences.remove(n)),
            Err(_) => None
        }
    }

    fn search(&self, key: &str) -> Result<usize, usize> {
        self.sequences.binary_search_by_key(&key, |&(ref k, _)| &k.as_ref())
    }
}

impl<K: AsRef<str>, V> From<Vec<(K, V)>> for SequenceMap<K, V> {
    /// Creates a `SequenceMap` from a `Vec` of key-value pairs.
    ///
    /// The input `Vec` will be sorted and deduplicated.
    ///
    /// If two elements exist with the same key, the first element is used.
    fn from(mut sequences: Vec<(K, V)>) -> SequenceMap<K, V> {
        sequences.sort_by(|a, b| a.0.as_ref().cmp(b.0.as_ref()));
        sequences.dedup_by(|a, b| a.0.as_ref() == b.0.as_ref());

        SequenceMap{sequences}
    }
}

impl<K: AsRef<str>, V> FromIterator<(K, V)> for SequenceMap<K, V> {
    /// Creates a `SequenceMap` from an iterator of key-value pairs.
    ///
    /// If two elements exist with the same key, the last element is used.
    fn from_iter<I: IntoIterator<Item=(K, V)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut map = SequenceMap::with_capacity(iter.size_hint().0);

        for (k, v) in iter {
            map.insert(k, v);
        }

        map
    }
}

/// A view into a single entry of a `SequenceMap`, which may be either occupied
/// or vacant.
///
/// This value is returned from the [`SequenceMap::entry`] method.
///
/// [`SequenceMap::entry`]: struct.SequenceMap.html#method.entry
pub enum Entry<'a, K: 'a, V: 'a> {
    /// An occupied entry
    Occupied(OccupiedEntry<'a, K, V>),
    /// A vacant entry
    Vacant(VacantEntry<'a, K, V>),
}

/// A view into an occupied entry in a `SequenceMap`.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut SequenceMap<K, V>,
    index: usize,
}

/// A view into a vacant entry in a `SequenceMap`.
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    map: &'a mut SequenceMap<K, V>,
    key: K,
    index: usize,
}

impl<'a, K, V> Entry<'a, K, V> {
    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(mut ent) => {
                f(ent.get_mut());
                Entry::Occupied(ent)
            }
            Entry::Vacant(ent) => Entry::Vacant(ent)
        }
    }

    /// Returns a mutable reference to the entry value,
    /// inserting the provided default if the entry is vacant.
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(ent) => ent.into_mut(),
            Entry::Vacant(ent) => ent.insert(default)
        }
    }

    /// Returns a mutable reference to the entry value,
    /// inserting a value using the provided closure if the entry is vacant.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Entry::Occupied(ent) => ent.into_mut(),
            Entry::Vacant(ent) => ent.insert(default())
        }
    }

    /// Returns a borrowed reference to the entry key.
    pub fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref ent) => ent.key(),
            Entry::Vacant(ref ent) => ent.key(),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// Returns a borrowed reference to the entry key.
    pub fn key(&self) -> &K {
        &self.map.sequences[self.index].0
    }

    /// Returns a borrowed reference to the entry value.
    pub fn get(&self) -> &V {
        &self.map.sequences[self.index].1
    }

    /// Returns a mutable reference to the entry value.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.sequences[self.index].1
    }

    /// Converts the `OccupiedEntry` into a mutable reference whose lifetime
    /// is bound to the `SequenceMap`.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.sequences[self.index].1
    }

    /// Replaces the entry value with the given value, returning the previous value.
    pub fn insert(&mut self, value: V) -> V {
        replace(self.get_mut(), value)
    }

    /// Removes the entry and returns the value.
    pub fn remove(self) -> V {
        self.map.sequences.remove(self.index).1
    }

    /// Removes the entry and returns the key-value pair.
    pub fn remove_entry(self) -> (K, V) {
        self.map.sequences.remove(self.index)
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// Returns a borrowed reference to the entry key.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Consumes the `VacantEntry` and returns ownership of the key.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Consumes the `VacantEntry` and inserts a value, returning a mutable
    /// reference to its place in the `SequenceMap`.
    pub fn insert(self, value: V) -> &'a mut V {
        self.map.sequences.insert(self.index, (self.key, value));
        &mut self.map.sequences[self.index].1
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for Entry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Entry::Occupied(ref ent) =>
                f.debug_tuple("Entry")
                    .field(ent)
                    .finish(),
            Entry::Vacant(ref ent) =>
                f.debug_tuple("Entry")
                    .field(ent)
                    .finish()
        }
    }
}

impl<'a, K: fmt::Debug, V: fmt::Debug> fmt::Debug for OccupiedEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

impl<'a, K: fmt::Debug, V> fmt::Debug for VacantEntry<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("VacantEntry")
            .field(self.key())
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::{FindResult, SequenceMap};

    #[test]
    fn test_seq_map_get() {
        let mut m = SequenceMap::new();

        m.insert("ab", 0);
        m.insert("ac", 1);

        assert_eq!(m.get("a").cloned(), None);
        assert_eq!(m.get("aa").cloned(), None);
        assert_eq!(m.get("ab").cloned(), Some(0));
        assert_eq!(m.get("ac").cloned(), Some(1));
    }

    #[test]
    fn test_seq_map_find() {
        let mut m = SequenceMap::new();

        m.insert("a", 0);
        m.insert("abcd", 1);

        m.insert("bcd", 2);
        m.insert("bce", 3);

        m.insert("cd", 4);
        m.insert("cde", 5);
        m.insert("cdf", 6);

        assert_eq!(m.find("a").cloned(), FindResult::Undecided(0));
        assert_eq!(m.find("ab").cloned(), FindResult::Incomplete);
        assert_eq!(m.find("abc").cloned(), FindResult::Incomplete);
        assert_eq!(m.find("abcd").cloned(), FindResult::Found(1));

        assert_eq!(m.find("b").cloned(), FindResult::Incomplete);
        assert_eq!(m.find("bc").cloned(), FindResult::Incomplete);
        assert_eq!(m.find("bcd").cloned(), FindResult::Found(2));
        assert_eq!(m.find("bce").cloned(), FindResult::Found(3));

        assert_eq!(m.find("c").cloned(), FindResult::Incomplete);
        assert_eq!(m.find("cd").cloned(), FindResult::Undecided(4));
        assert_eq!(m.find("cde").cloned(), FindResult::Found(5));
        assert_eq!(m.find("cdf").cloned(), FindResult::Found(6));

        assert_eq!(m.find("d").cloned(), FindResult::NotFound);
    }

    #[test]
    fn test_seq_map_insert() {
        let mut m = SequenceMap::new();

        assert_eq!(m.insert("a", 0), None);
        assert_eq!(m.insert("b", 1), None);
        assert_eq!(m.insert("a", 2), Some(0));
    }

    #[test]
    fn test_seq_map_entry() {
        let mut m = SequenceMap::new();

        assert_eq!(*m.entry("a").or_insert(0), 0);
        assert_eq!(m.get("a").cloned(), Some(0));
        assert_eq!(*m.entry("a").or_insert(1), 0);
    }

    #[test]
    fn test_seq_map_from() {
        let m: SequenceMap<&'static str, u32> = [
            ("foo", 0),
            ("bar", 1),
            ("baz", 2),
            ("foo", 3),
        ].iter().cloned().collect();

        assert_eq!(m.sequences(), [
            ("bar", 1),
            ("baz", 2),
            ("foo", 3),
        ]);

        let m = SequenceMap::from(vec![
            ("foo", 0),
            ("bar", 1),
            ("baz", 2),
            ("foo", 3),
        ]);

        assert_eq!(m.sequences(), [
            ("bar", 1),
            ("baz", 2),
            ("foo", 0),
        ]);
    }

    #[test]
    fn test_seq_map_types() {
        // Ensure that many str-like types can serve as SequenceMap key type
        use std::borrow::Cow;
        use std::rc::Rc;
        use std::sync::Arc;

        struct Foo<'a> {
            _a: SequenceMap<&'a str, ()>,
            _b: SequenceMap<Cow<'a, str>, ()>,
        }

        let _ = Foo{
            _a: SequenceMap::new(),
            _b: SequenceMap::new(),
        };

        SequenceMap::<&'static str, ()>::new();
        SequenceMap::<String, ()>::new();
        SequenceMap::<Cow<'static, str>, ()>::new();
        SequenceMap::<Box<str>, ()>::new();
        SequenceMap::<Rc<str>, ()>::new();
        SequenceMap::<Arc<str>, ()>::new();
    }
}
