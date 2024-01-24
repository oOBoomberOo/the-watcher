use std::{collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMap<K: Hash + Eq, V, T> {
    items: HashMap<K, V>,
    timetable: HashMap<K, T>,
}

impl<K, V, T> TimingMap<K, V, T>
where
    K: Hash + Eq,
    T: Ord,
{
    pub fn new(content: HashMap<K, V>) -> Self {
        Self {
            items: content,
            timetable: HashMap::new(),
        }
    }

    pub fn is_scheduled(&self, key: K) -> bool {
        self.timetable.contains_key(&key)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn contains(&self, key: K) -> bool {
        self.items.contains_key(&key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.items.insert(key, value);
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        self.timetable.remove(&key);
        self.items.remove(&key)
    }

    pub fn refresh(&mut self, key: K, time: T) -> Option<T> {
        self.timetable.insert(key, time)
    }

    pub fn refresh_and_insert(&mut self, key: K, value: V, time: T) -> Option<V>
    where
        K: Clone,
    {
        self.timetable.insert(key.clone(), time);
        self.items.insert(key, value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.items.get(&key)
    }

    pub fn drain_expired(&mut self, time: T) -> impl Iterator<Item = (&K, &V)> + '_
    where
        HashMap<K, V>: Clone,
    {
        let (drained_items, remaining_items) =
            self.timetable.drain().partition(|(_, t)| t <= &time);
        self.timetable = remaining_items;

        let content = &self.items;

        drained_items
            .into_keys()
            .filter_map(|k| content.get_key_value(&k))
    }

    pub fn unused_items(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items
            .iter()
            .filter(|(key, _)| !self.timetable.contains_key(key))
    }

    pub fn extend_timings(&mut self, timings: impl IntoIterator<Item = (K, T)>) {
        self.extend(timings)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items.iter()
    }

    pub fn iter_timings(&self) -> impl Iterator<Item = (&K, &T)> {
        self.timetable.iter()
    }
}

impl<K, V, T> Default for TimingMap<K, V, T>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self {
            items: HashMap::default(),
            timetable: HashMap::default(),
        }
    }
}

impl<K, V, T> FromIterator<(K, V)> for TimingMap<K, V, T>
where
    K: Hash + Eq,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
            timetable: HashMap::new(),
        }
    }
}

impl<K, V, T> FromIterator<(K, V, T)> for TimingMap<K, V, T>
where
    K: Hash + Eq + Clone,
    T: Ord,
    TimingMap<K, V, T>: Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V, T)>>(iter: I) -> Self {
        let mut map = Self::default();
        for (key, value, time) in iter {
            map.refresh_and_insert(key, value, time);
        }
        map
    }
}

impl<K, V, T> Extend<(K, T)> for TimingMap<K, V, T>
where
    K: Hash + Eq,
{
    fn extend<I: IntoIterator<Item = (K, T)>>(&mut self, iter: I) {
        self.timetable.extend(iter.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    macro_rules! collect {
        ($iter:expr) => {{
            $iter
                .map(|(k, v)| (*k, *v))
                .sorted_by(|(a_key, _), (b_key, _)| ::std::cmp::Ord::cmp(a_key, b_key))
                .collect_vec()
        }};
    }

    #[test]
    fn insert_and_expiration() {
        let mut map: TimingMap<char, &'static str, i32> = TimingMap::default();
        map.insert('A', "Ina Norman");
        map.insert('B', "Mabelle Byrd");
        map.insert('C', "Michael Stokes");

        map.refresh('A', 1);
        map.refresh('B', 1);
        map.refresh('C', 3);

        let expired_items = collect! { map.drain_expired(1) };
        assert_eq!(
            expired_items,
            vec![('A', "Ina Norman"), ('B', "Mabelle Byrd")]
        );

        let expired_items = collect! { map.drain_expired(2) };
        assert_eq!(expired_items, vec![], "no items should be expired yet");

        let expired_items = collect! { map.drain_expired(3) };
        assert_eq!(
            expired_items,
            vec![('C', "Michael Stokes")],
            "final remaining item should be expired"
        );
    }

    #[test]
    fn removed_item_should_not_show_up() {
        let mut map: TimingMap<char, &'static str, i32> = TimingMap::default();
        map.insert('A', "Ina Norman");
        map.refresh('A', 1);

        let expired_items = collect! { map.drain_expired(1) };
        assert_eq!(expired_items, vec![('A', "Ina Norman")]);

        map.remove('A');
        map.refresh('A', 2);

        let expired_items = collect! { map.drain_expired(2) };
        assert_eq!(expired_items, vec![]);
    }
}
