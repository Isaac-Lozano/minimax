use lru_cache::LruCache;

use std::hash::Hash;

#[derive(Clone)]
pub struct TranspositionTable<B, M>
    where B: Eq + Hash
{
    cache: LruCache<B, (M, u32)>,
}

impl<B, M> TranspositionTable<B, M>
    where B: Eq + Hash,
          M: Clone
{
    pub fn new(capacity: usize) -> TranspositionTable<B, M>
    {
        TranspositionTable {
            cache: LruCache::new(capacity),
        }
    }

    pub fn get(&mut self, board: &B, depth: u32) -> Option<M>
    {
        if let Some(precomputed_move) = self.cache.get_mut(board)
        {
            if precomputed_move.1 >= depth
            {
                return Some(precomputed_move.0.clone());
            }
        }

        None
    }

    pub fn insert(&mut self, board: B, mv: M, depth: u32)
    {
        if let Some(precomputed_move) = self.cache.get_mut(&board)
        {
            if precomputed_move.1 >= depth
            {
                return;
            }
        }
        self.cache.insert(board, (mv, depth));
    }
}

#[cfg(test)]
mod tests {
    use TranspositionTable;
    fn test_tt() {
        let mut tt = TranspositionTable::new(2);
        tt.insert(3, 6, 8);
        tt.insert(3, 4, 7);
        tt.insert(2, 3, 7);
        assert_eq!(tt.get(&3, 3), Some(6));
        assert_eq!(tt.get(&3, 9), None);
        assert_eq!(tt.get(&2, 2), Some(3));
        tt.insert(1, 2, 3);
        assert_eq!(tt.get(&1, 2), Some(2));
        assert_eq!(tt.get(&3, 3), None);
    }
}
