pub mod board;

use board::Board;

use std::i32;
use std::ops::Neg;
use std::collections::HashMap;

#[derive(Copy,Clone,Debug)]
pub enum Team
{
    Enemy,
    Ally,
}

impl Team
{
    pub fn other_team(self) -> Team
    {
        match self
        {
            Team::Enemy => Team::Ally,
            Team::Ally => Team::Enemy,
        }
    }
}

#[derive(PartialEq,Eq,PartialOrd,Ord,Copy,Clone,Debug)]
pub enum Score
{
    Lose,
    Heuristic(i32),
    Win,
}

impl Neg for Score
{
    type Output = Self;
    fn neg(self) -> Self
    {
        match self
        {
            Score::Win => Score::Lose,
            Score::Lose => Score::Win,
            Score::Heuristic(val) => Score::Heuristic(-val),
        }
    }
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MoveStats<M>
{
    pub mv: Option<M>,
    pub score: Score,
    pub turns: u32,
    pub nodes_visited: u64,
}

#[derive(Clone,Debug)]
pub struct Minimax<B: Board>
{
    ally_move_cache: HashMap<B, Vec<B::Move>>,
    enemy_move_cache: HashMap<B, Vec<B::Move>>,
}

impl<B: Board> Minimax<B>
{
    pub fn new() -> Minimax<B>
    {
        Minimax
        {
            ally_move_cache: HashMap::new(),
            enemy_move_cache: HashMap::new(),
        }
    }

    /// Minimax driver function.
    ///
    /// `turn` is the current player.
    pub fn minimax(&mut self, board: &B, turn: Team, plies: u32) -> MoveStats<B::Move>
    {
        match turn
        {
            Team::Ally =>
                self.max(board, plies),
            Team::Enemy =>
                self.min(board, plies),
        }
    }

    /// Generates best move for ally
    fn max(&mut self, board: &B, plies: u32) -> MoveStats<B::Move>
    {
        let moves;
//        if let Some(cached_moves) = self.ally_move_cache.get(board).map(|m| m.to_owned())
//        {
//            moves = cached_moves;
//        }
//        else
//        {
            moves = board.gen_ally_moves();
//            self.ally_move_cache.insert(board.clone(), moves.clone());
//        }

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return MoveStats
            {
                mv: None,
                score: Score::Lose,
                turns: 0,
                nodes_visited: 0,
            };
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            MoveStats
            {
                mv: None,
                score: board.score(),
                turns: 0,
                nodes_visited: 0,
            }
        }
        else
        {
            let mut best = MoveStats {
                mv: None,
                score: Score::Lose,
                turns: 0,
                nodes_visited: 0,
            };

            for mv in moves
            {
                if plies == 7
                {
//                    println!("Move {:#?}", mv);
                }

                /* Make a clone of the board so we don't break this one */
                let mut board_clone = board.clone();
                board_clone.do_move(&mv);
    
                /* Find enemy's best move */
                let enemy_move = self.min(&board_clone, plies - 1);
                if plies == 7
                {
//                    println!("ENEMY {:#?}", enemy_move);
                }
    
                /* TODO: Try to postpone losing */
                if best.mv.is_none() || enemy_move.score > best.score || (enemy_move.score == best.score && enemy_move.turns < best.turns)
                {
                    best.mv = Some(mv);
                    best.score = enemy_move.score;
                    best.turns = enemy_move.turns + 1;
                }
                best.nodes_visited += enemy_move.nodes_visited + 1;
            }
            best
        }
    }

    /// Generates best move for enemy
    fn min(&mut self, board: &B, plies: u32) -> MoveStats<B::Move>
    {
        let moves;
//        if let Some(cached_moves) = self.enemy_move_cache.get(board).map(|m| m.to_owned())
//        {
//            moves = cached_moves;
//        }
//        else
//        {
            moves = board.gen_enemy_moves();
//            self.enemy_move_cache.insert(board.clone(), moves.clone());
//        }

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return MoveStats
            {
                mv: None,
                /* If enemy can't move, we win. */
                score: Score::Win,
                turns: 0,
                nodes_visited: 0,
            };
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            MoveStats
            {
                mv: None,
                score: board.score(),
                turns: 0,
                nodes_visited: 0,
            }
        }
        else
        {
            let mut best = MoveStats {
                mv: None,
                /* Technically doesn't matter, but for consistancy's sake */
                score: Score::Win,
                turns: 0,
                nodes_visited: 0,
            };

            for mv in moves
            {
                /* Make a clone of the board so we don't break this one */
                let mut board_clone = board.clone();
                board_clone.do_move(&mv);

                /* Find ally's best move */
                let ally_move = self.max(&board_clone, plies - 1);

                /* TODO: Try to postpone losing */
                if best.mv.is_none() || ally_move.score < best.score || (ally_move.score == best.score && ally_move.turns < best.turns)
                {
                    best.mv = Some(mv);
                    best.score = ally_move.score;
                    best.turns = ally_move.turns + 1;
                }
                best.nodes_visited += ally_move.nodes_visited + 1;
            }
            best
        }
    }
}

#[test]
fn test_score_ord()
{
    assert!(Score::Win > Score::Heuristic(0));
    assert!(Score::Heuristic(0) > Score::Lose);
    assert!(Score::Win > Score::Lose);
    assert!(Score::Heuristic(100) > Score::Heuristic(0));
    assert!(Score::Heuristic(0) > Score::Heuristic(-100));
    assert!(Score::Win == Score::Win);
    assert!(Score::Lose == Score::Lose);
    assert!(Score::Heuristic(0) == Score::Heuristic(0));
}
