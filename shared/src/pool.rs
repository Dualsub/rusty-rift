#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub struct PoolIndex {
    index: u32,
    generation: u32,
}

impl PoolIndex {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

pub struct Pool<T> {
    elements: Vec<Option<T>>,
    generations: Vec<u32>,
    free_slots: Vec<u32>,
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            generations: Vec::new(),
            free_slots: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len() - self.free_slots.len()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
            free_slots: Vec::new(),
        }
    }

    pub fn push(&mut self, element: T) -> PoolIndex {
        if let Some(index) = self.free_slots.pop() {
            self.elements[index as usize] = Some(element);
            let generation = self.generations[index as usize].wrapping_add(1);
            self.generations[index as usize] = generation;

            PoolIndex { index, generation }
        } else {
            let index = self.elements.len() as u32;
            self.elements.push(Some(element));
            self.generations.push(1);

            PoolIndex {
                index,
                generation: 1,
            }
        }
    }

    pub fn get(&self, index: PoolIndex) -> Option<&T> {
        if let Some(element) = self.elements.get(index.index as usize) {
            if self.generations[index.index as usize] == index.generation {
                return element.as_ref();
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: PoolIndex) -> Option<&mut T> {
        if let Some(element) = self.elements.get_mut(index.index as usize) {
            if self.generations[index.index as usize] == index.generation {
                return element.as_mut();
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: PoolIndex) -> Option<T> {
        if let Some(element) = self.elements.get_mut(index.index as usize) {
            let generation = self.generations[index.index as usize];
            if generation == index.generation {
                self.generations[index.index as usize] = generation.wrapping_add(1);
                self.free_slots.push(index.index);
                return element.take();
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (PoolIndex, &T)> {
        self.elements
            .iter()
            .zip(&self.generations)
            .enumerate()
            .filter_map(|(i, (opt, g))| {
                opt.as_ref().map(|v| {
                    (
                        PoolIndex {
                            index: i as u32,
                            generation: *g,
                        },
                        v,
                    )
                })
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (PoolIndex, &mut T)> {
        self.elements
            .iter_mut()
            .zip(&self.generations)
            .enumerate()
            .filter_map(|(i, (opt, g))| {
                opt.as_mut().map(|v| {
                    (
                        PoolIndex {
                            index: i as u32,
                            generation: *g,
                        },
                        v,
                    )
                })
            })
    }
}

// Generated tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_get_remove_basic() {
        let mut pool = Pool::new();

        let a = pool.push(10);
        let b = pool.push(20);

        assert_eq!(pool.get(a), Some(&10));
        assert_eq!(pool.get(b), Some(&20));

        let removed = pool.remove(a);
        assert_eq!(removed, Some(10));
        assert_eq!(pool.get(a), None); // stale now

        // b still alive
        assert_eq!(pool.get(b), Some(&20));
    }

    #[test]
    fn reuse_index_and_bump_generation() {
        let mut pool = Pool::new();

        let h1 = pool.push(111);
        let idx = h1.index();
        let gen1 = h1.generation();

        assert_eq!(pool.remove(h1), Some(111));
        assert_eq!(pool.get(h1), None); // stale

        // Next push should reuse the freed index and bump generation
        let h2 = pool.push(222);
        assert_eq!(h2.index(), idx);
        assert_ne!(h2.generation(), gen1);
        assert_eq!(pool.get(h2), Some(&222));
    }

    #[test]
    fn get_mut_updates_value() {
        let mut pool = Pool::new();
        let h = pool.push(5);
        if let Some(x) = pool.get_mut(h) {
            *x *= 3;
        }
        assert_eq!(pool.get(h), Some(&15));
    }

    #[test]
    fn remove_is_move_not_clone() {
        #[derive(Debug, PartialEq, Eq)]
        struct NonClone(i32);
        let mut pool = Pool::new();
        let h = pool.push(NonClone(7));
        // Move out
        let v = pool.remove(h);
        assert_eq!(v, Some(NonClone(7)));
        // Now slot is free; handle is stale
        assert_eq!(pool.get(h), None);
    }

    #[test]
    fn iter_sees_only_live_elements() {
        let mut pool = Pool::new();
        let _h1 = pool.push(1);
        let h2 = pool.push(2);
        let _h3 = pool.push(3);

        // Remove middle
        assert_eq!(pool.remove(h2), Some(2));

        let items: Vec<(u32, u32, i32)> = pool
            .iter()
            .map(|(h, v)| (h.index(), h.generation(), *v))
            .collect();

        // Should see 1 and 3 only
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|(_, _, v)| *v == 1));
        assert!(items.iter().any(|(_, _, v)| *v == 3));
        // Old handle h2 is stale
        assert_eq!(pool.get(h2), None);
    }

    #[test]
    fn iter_mut_can_mutate_all_live() {
        let mut pool = Pool::new();
        let _h1 = pool.push(2);
        let h2 = pool.push(3);
        let _h3 = pool.push(5);

        // Remove one so we test skipping frees
        assert!(pool.remove(h2).is_some());

        for (_h, v) in pool.iter_mut() {
            *v *= 10;
        }

        // Collect via iter (immutable) to verify
        let vals: Vec<i32> = pool.iter().map(|(_, v)| *v).collect();
        assert_eq!(vals.len(), 2);
        assert!(vals.contains(&20));
        assert!(vals.contains(&50));
    }

    #[test]
    fn with_capacity_basic_ops_and_reuse() {
        let mut pool: Pool<i32> = Pool::with_capacity(128);

        // Fill a few
        let hs: Vec<_> = (0..10).map(|i| pool.push(i as i32)).collect();
        for (i, h) in hs.iter().enumerate() {
            assert_eq!(*pool.get(*h).unwrap(), i as i32);
        }

        // Remove some and ensure handles go stale
        for (i, h) in hs.iter().enumerate().step_by(2) {
            assert_eq!(pool.remove(*h), Some(i as i32));
            assert!(pool.get(*h).is_none()); // stale
        }

        // Push again—should reuse freed slots and still work
        let g = pool.push(123);
        assert_eq!(pool.get(g), Some(&123));
    }

    #[test]
    fn stale_handle_after_reuse_is_rejected() {
        let mut pool = Pool::new();
        let h1 = pool.push(1);
        assert_eq!(pool.remove(h1), Some(1));
        let h2 = pool.push(2);
        // Same index reused, but generation changed; old handle must not access new value
        if h1.index() == h2.index() {
            assert_eq!(pool.get(h1), None);
            assert_eq!(pool.get(h2), Some(&2));
        }
    }
}
