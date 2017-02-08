use ::Score;

use std::fmt;
use std::hash::Hash;

pub trait Board: Clone + Eq + Hash
{
    type Move: Clone + fmt::Debug;
    fn gen_ally_moves(&self) -> Vec<Self::Move>;
    fn gen_enemy_moves(&self) -> Vec<Self::Move>;
    fn do_move(&mut self, mv: &Self::Move);
    fn score(&self) -> Score;
    fn is_game_over(&self) -> bool;
}
