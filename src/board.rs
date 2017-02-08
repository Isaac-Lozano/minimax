use ::Score;

use std::fmt;

pub trait Board: Clone
{
    type Move: fmt::Debug;
    fn gen_ally_moves(&self) -> Vec<Self::Move>;
    fn gen_enemy_moves(&self) -> Vec<Self::Move>;
    fn do_move(&mut self, mv: &Self::Move);
    fn score(&self) -> Score;
    fn is_game_over(&self) -> bool;
}
