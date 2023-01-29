extern crate lru;

pub mod board;
pub mod transposition_table;

use board::Board;
use transposition_table::TranspositionTable;

use std::i32;
use std::fmt;
use std::ops::Neg;
use std::hash::Hash;
use std::cmp::Ordering;
use std::num::NonZeroUsize;

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

#[derive(PartialEq,Eq,PartialOrd,Ord,Copy,Clone,Debug,Hash)]
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

#[derive(PartialEq,Eq,Copy,Clone,Debug,Hash)]
pub struct TimedScore
{
    pub score: Score,
    pub turns: u32,
}

impl PartialOrd for TimedScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimedScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.score.cmp(&other.score) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => {
                match self.score.cmp(&Score::Heuristic(0)) {
                    Ordering::Less => self.turns.cmp(&other.turns),
                    Ordering::Equal => Ordering::Equal,
                    Ordering::Greater => other.turns.cmp(&self.turns)
                }
            }
            Ordering::Greater => Ordering::Greater,
        }
    }
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MoveStats<M>
{
    pub mv: Option<M>,
    pub score: TimedScore,
    pub nodes_visited: u64,
    pub mvs: Vec<M>,
}

pub struct Minimax<B>
    where B: Board + Eq + Hash
{
    ally_ttable: TranspositionTable<B, MoveStats<B::Move>>,
    enemy_ttable: TranspositionTable<B, MoveStats<B::Move>>,
}

impl<B> Minimax<B>
    where B: Board + Eq + Hash
{
    pub fn new(ttable_size: NonZeroUsize) -> Minimax<B>
    {
        Minimax
        {
            ally_ttable: TranspositionTable::new(ttable_size),
            enemy_ttable: TranspositionTable::new(ttable_size),
        }
    }

    /// Minimax driver function.
    ///
    /// `turn` is the current player.
    pub fn minimax(&mut self, board: &B, turn: Team, plies: u32) -> MoveStats<B::Move>
    where <B as Board>::Move: fmt::Display
    {
        let lose = TimedScore {
            score: Score::Lose,
            turns: 0,
        };
        let win = TimedScore {
            score: Score::Win,
            turns: 0,
        };

        let mut optimal_move = match turn
        {
            Team::Ally =>
                self.max(board, plies, lose, win),
            Team::Enemy =>
                self.min(board, plies, lose, win),
        };

        optimal_move.nodes_visited += 1;
        optimal_move
    }

    /// Generates best move for ally
    fn max(&mut self, board: &B, plies: u32, mut alpha: TimedScore, beta: TimedScore) -> MoveStats<B::Move>
    where <B as Board>::Move: fmt::Display
    {
        let moves = board.gen_ally_moves();

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return MoveStats
            {
                mv: None,
                score: TimedScore {
                    score: Score::Lose,
                    turns: 0,
                },
                nodes_visited: 0,
                mvs: Vec::new(),
            };
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            return MoveStats
            {
                mv: None,
                score: TimedScore {
                    score: board.score(),
                    turns: 0,
                },
                nodes_visited: 0,
                mvs: Vec::new(),
            }
        }

        let mut best = MoveStats{
            mv: None,
            score: TimedScore {
                score: Score::Lose,
                turns: 0,
            },
            nodes_visited: 0,
            mvs: Vec::new(),
        };

        if let Some(mut precomputed_move) = self.ally_ttable.get(board, plies)
        {
            precomputed_move.mvs = Vec::new();
            return precomputed_move;
        }

        for mv in moves
        {
            /* Make a clone of the board so we don't break this one */
            let mut board_clone = board.clone();
            board_clone.do_move(&mv);
    
            /* Find enemy's best move */
            let enemy_move = self.min(&board_clone, plies - 1, alpha, beta);

//            if plies == 5 {
//                println!("  my move: {}", mv);
//                println!("  score: {:?}", enemy_move.score);
//                println!("  a: {:?} b {:?}", alpha, beta);
//            }

            if best.mv.is_none() || enemy_move.score > best.score
            {
                best.mv = Some(mv);
                best.score = enemy_move.score;
                best.score.turns += 1;
                best.mvs = enemy_move.mvs.clone();
            }

            best.nodes_visited += enemy_move.nodes_visited + 1;

            /* Set α and break on β ≤ α */
            if best.score > alpha
            {
                alpha = best.score;
            }
            if alpha >= beta
            {
                if plies == 5 {
                    println!("  PRUNED");
                }
                break;
            }
        }

        best.mvs.push(best.mv.clone().unwrap());
        self.ally_ttable.insert(board.clone(), best.clone(), plies);

        best
    }

    /// Generates best move for enemy
    fn min(&mut self, board: &B, plies: u32, alpha: TimedScore, mut beta: TimedScore) -> MoveStats<B::Move>
    where <B as Board>::Move: fmt::Display
    {
        let moves = board.gen_enemy_moves();

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return MoveStats
            {
                mv: None,
                /* If enemy can't move, we win. */
                score: TimedScore {
                    score: Score::Win,
                    turns: 0,
                },
                nodes_visited: 0,
                mvs: Vec::new(),
            };
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            return MoveStats
            {
                mv: None,
                score: TimedScore {
                    score: board.score(),
                    turns: 0,
                },
                nodes_visited: 0,
                mvs: Vec::new(),
            }
        }

        let mut best = MoveStats {
            mv: None,
            /* Technically doesn't matter, but for consistancy's sake */
            score: TimedScore {
                score: Score::Win,
                turns: 0,
            },
            nodes_visited: 0,
            mvs: Vec::new(),
        };

        if let Some(precomputed_move) = self.enemy_ttable.get(board, plies)
        {
            return precomputed_move;
        }

        for mv in moves
        {
            /* Make a clone of the board so we don't break this one */
            let mut board_clone = board.clone();
            board_clone.do_move(&mv);

            if plies == 6 {
                println!("ENEMY {}", mv);
            }

            /* Find ally's best move */
            let ally_move = self.max(&board_clone, plies - 1, alpha, beta);

            if best.mv.is_none() || ally_move.score < best.score
            {
                best.mv = Some(mv);
                best.score = ally_move.score;
                best.score.turns += 1;
                best.mvs = ally_move.mvs.clone();
            }

            best.nodes_visited += ally_move.nodes_visited + 1;

            /* Set β and break on β ≤ α */
            if best.score < beta
            {
                beta = best.score;
            }
            if beta <= alpha
            {
                break;
            }
        }

        best.mvs.push(best.mv.clone().unwrap());
        self.enemy_ttable.insert(board.clone(), best.clone(), plies);

        best
    }
}

#[cfg(test)]
mod tests
{
    use super::{Team, Score, Minimax, MoveStats};
    use board::Board;
    use std::num::NonZeroUsize;

    #[derive(Clone,PartialEq,Eq,Debug)]
    struct SimpleMove(usize);
    #[derive(Clone,PartialEq,Eq,Hash,Debug)]
    enum SimpleBoard
    {
        Node(Vec<SimpleBoard>),
        Leaf(Score),
    }

    /* These aren't really efficient, but they
     * work for testing.
     */
    impl Board for SimpleBoard
    {
        type Move = SimpleMove;

        fn gen_ally_moves(&self) -> Vec<Self::Move>
        {
            match *self
            {
                SimpleBoard::Node(ref v) =>
                    (0..v.len()).map(|u| SimpleMove(u)).collect(),
                SimpleBoard::Leaf(_) =>
                    /* Trick the algorithm into thinking
                     * we still have moves left
                     */
                    vec![SimpleMove(1)],
            }
        }

        fn gen_enemy_moves(&self) -> Vec<Self::Move>
        {
            match *self
            {
                SimpleBoard::Node(ref v) =>
                    (0..v.len()).map(|u| SimpleMove(u)).collect(),
                SimpleBoard::Leaf(_) =>
                    /* Trick the algorithm into thinking
                     * we still have moves left
                     */
                    vec![SimpleMove(1)],
            }
        }

        fn do_move(&mut self, mv: &Self::Move)
        {
            let child = match *self
            {
                SimpleBoard::Node(ref v) =>
                    Some(v[mv.0].clone()),
                SimpleBoard::Leaf(_) =>
                    None,
            };

            if let Some(c) = child
            {
                *self = c;
            }
        }

        fn score(&self) -> Score
        {
            match *self
            {
                SimpleBoard::Node(_) =>
                    unreachable!(),
                SimpleBoard::Leaf(s) =>
                    s,
            }
        }

        fn is_game_over(&self) -> bool
        {
            match *self
            {
                SimpleBoard::Node(_) =>
                    false,
                SimpleBoard::Leaf(_) =>
                    true,
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

    #[test]
    fn test_move_stats_ord()
    {
        assrt!()
    }

    #[test]
    fn test_minimax()
    {
        let mut minimax = Minimax::new(NonZeroUsize::new(100).unwrap());
        let game1 =
        SimpleBoard::Node(vec![
            SimpleBoard::Node(vec![
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(5)),
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(7)),
                        SimpleBoard::Leaf(Score::Heuristic(4)),
                        SimpleBoard::Leaf(Score::Heuristic(5)),
                    ]),
                ]),
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(3)),
                    ]),
                ]),
            ]),
            SimpleBoard::Node(vec![
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                        SimpleBoard::Leaf(Score::Heuristic(9)),
                    ]),
                ]),
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(7)),
                    ]),
                ]),
            ]),
            SimpleBoard::Node(vec![
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(5)),
                    ]),
                ]),
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(9)),
                        SimpleBoard::Leaf(Score::Heuristic(8)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                    ]),
                ]),
            ]),
        ]);
        let game2 =
        SimpleBoard::Node(vec![
            SimpleBoard::Node(vec![
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(12)),
                        SimpleBoard::Leaf(Score::Heuristic(10)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(-18)),
                        SimpleBoard::Leaf(Score::Heuristic(-7)),
                    ]),
                ]),
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                        SimpleBoard::Leaf(Score::Heuristic(-17)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(-3)),
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                    ]),
                ]),
            ]),
            SimpleBoard::Node(vec![
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(-19)),
                        SimpleBoard::Leaf(Score::Heuristic(-16)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(-4)),
                        SimpleBoard::Leaf(Score::Heuristic(-6)),
                    ]),
                ]),
                SimpleBoard::Node(vec![
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(-1)),
                        SimpleBoard::Leaf(Score::Heuristic(6)),
                    ]),
                    SimpleBoard::Node(vec![
                        SimpleBoard::Leaf(Score::Heuristic(9)),
                        SimpleBoard::Leaf(Score::Heuristic(15)),
                    ]),
                ]),
            ]),
        ]);

        println!();
        println!("Game 1");
        assert_eq!(game1.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1), SimpleMove(2)]);
        let move_stats1 = minimax.minimax(&game1, Team::Ally, 4);
        let optimal_move1 = MoveStats {
            mv: Some(SimpleMove(1)),
            score: Score::Heuristic(6),
            turns: 4,
            nodes_visited: 25,
        };
        assert_eq!(move_stats1, optimal_move1);

        println!();
        println!("Game 2");
        assert_eq!(game2.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1)]);
        let move_stats2 = minimax.minimax(&game2, Team::Ally, 4);
        let optimal_move2 = MoveStats {
            mv: Some(SimpleMove(0)),
            score: Score::Heuristic(-3),
            turns: 4,
            nodes_visited: 21,
        };
        assert_eq!(move_stats2, optimal_move2);

        println!();
        println!("Testing caching");
        assert_eq!(game2.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1)]);
        let move_stats2 = minimax.minimax(&game2, Team::Ally, 4);
        let optimal_move2 = MoveStats {
            mv: Some(SimpleMove(0)),
            score: Score::Heuristic(-3),
            turns: 4,
            nodes_visited: 21,
        };
        assert_eq!(move_stats2, optimal_move2);
    }
}
