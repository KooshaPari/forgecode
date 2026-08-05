use ::std::collections::HashMap;
use ::std::hash::Hash;

pub mod vec {

    use std::collections::HashMap;

    use super::Key;

    #[allow(unused)]
    pub fn unify_by_key<T: Key>(
        base: &mut Vec<T>,
        other: Vec<T>,
        mut merge_from: impl FnMut(&mut T, T),
    ) where
        T::Id: Clone + std::hash::Hash + Eq,
    {
        // Create a HashMap for O(1) lookup of base agents by their key
        let mut base_map: HashMap<T::Id, usize> = HashMap::new();
        for (index, agent) in base.iter().enumerate() {
            base_map.insert(agent.key().clone(), index);
        }

        for other_agent in other {
            if let Some(&index) = base_map.get(other_agent.key()) {
                // If the base contains an agent with the same Key, merge them
                if let Some(base_agent) = base.get_mut(index) {
                    merge_from(base_agent, other_agent);
                }
            } else {
                // Otherwise, append the other agent to the base list
                base.push(other_agent);
            }
        }
    }
}

#[allow(unused)]
pub trait Key {
    type Id: Eq;
    fn key(&self) -> &Self::Id;
}

#[allow(unused)]
pub fn hashmap<K: Eq + Hash, V>(base: &mut HashMap<K, V>, other: HashMap<K, V>) {
    for (key, value) in other {
        base.insert(key, value);
    }
}
