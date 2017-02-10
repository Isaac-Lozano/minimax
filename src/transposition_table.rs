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
        self.cache.insert(board, (mv, depth));
    }
}
