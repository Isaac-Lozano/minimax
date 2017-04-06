extern crate lru_cache;

pub mod board;
pub mod transposition_table;

use board::Board;
use transposition_table::TranspositionTable;

use std::i32;
use std::ops::Neg;
use std::hash::Hash;
use std::cmp::Ordering;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

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

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct MoveStats<M>
{
    pub mv: Option<M>,
    pub score: Score,
    pub turns: u32,
    pub nodes_visited: u64,
}

impl<M> MoveStats<M> {
    fn enemy_cmp(&self, other: &Self) -> Ordering {
        match self.score.cmp(&other.score) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => {
                match self.score.cmp(&Score::Heuristic(0)) {
                    Ordering::Less => other.turns.cmp(&self.turns),
                    Ordering::Equal => Ordering::Equal,
                    Ordering::Greater => self.turns.cmp(&other.turns)
                }
            }
            Ordering::Greater => Ordering::Greater,
        }
    }
}

impl<M> PartialOrd for MoveStats<M>
    where M: Eq
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<M> Ord for MoveStats<M>
    where M: Eq
{
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

pub struct Minimax<B>
    where B: Board + Eq + Hash
{
    ally_ttable: TranspositionTable<B, MoveStats<B::Move>>,
    enemy_ttable: TranspositionTable<B, MoveStats<B::Move>>,
    stopper: Receiver<()>,
}

impl<B> Minimax<B>
    where B: Board + Eq + Hash
{
    pub fn new(ttable_size: usize, stopper: Receiver<()>) -> Minimax<B>
    {
        Minimax
        {
            ally_ttable: TranspositionTable::new(ttable_size),
            enemy_ttable: TranspositionTable::new(ttable_size),
            stopper: stopper,
        }
    }

    /// Minimax driver function.
    ///
    /// `turn` is the current player.
    pub fn minimax(&mut self, board: &B, turn: Team, plies: u32) -> Option<MoveStats<B::Move>>
    {
        let mut optimal_move = match turn
        {
            Team::Ally => {
                self.max(board, plies, Score::Lose, Score::Win)
            }
            Team::Enemy => {
                self.min(board, plies, Score::Lose, Score::Win)
            }
        };

        if let Some(ref mut optimal) = optimal_move {
            optimal.nodes_visited += 1;
        }

        optimal_move
    }

    /// Generates best move for ally
    fn max(&mut self, board: &B, plies: u32, mut alpha: Score, beta: Score) -> Option<MoveStats<B::Move>>
    {
        match self.stopper.try_recv() {
            Ok(_) => return None,
            Err(e) => {
                match e {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => panic!("stopper channel disconnected"),
                }
            }
        }

        let moves = board.gen_ally_moves();

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return Some(MoveStats
            {
                mv: None,
                score: Score::Lose,
                turns: 0,
                nodes_visited: 0,
            });
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            return Some(MoveStats
            {
                mv: None,
                score: board.score(),
                turns: 0,
                nodes_visited: 0,
            });
        }

        let mut best = MoveStats {
            mv: None,
            score: Score::Lose,
            turns: 0,
            nodes_visited: 0,
        };

        if let Some(precomputed_move) = self.ally_ttable.get(board, plies)
        {
            return Some(precomputed_move);
        }

        for mv in moves
        {

            /* Make a clone of the board so we don't break this one */
            let mut board_clone = board.clone();
            board_clone.do_move(&mv);
    
            /* Find enemy's best move */
            let enemy_move = match self.min(&board_clone, plies - 1, alpha, beta) {
                Some(mv) => mv,
                None => return None,
            };

            if best.mv.is_none() || enemy_move > best
            {
                best.mv = Some(mv);
                best.score = enemy_move.score;
                best.turns = enemy_move.turns + 1;
            }

            best.nodes_visited += enemy_move.nodes_visited + 1;

            /* Set α and break on β ≤ α */
            if best.score > alpha
            {
                alpha = enemy_move.score;
            }
            if alpha >= beta
            {
                break;
            }
        }

        self.ally_ttable.insert(board.clone(), best.clone(), plies);

        Some(best)
    }

    /// Generates best move for enemy
    fn min(&mut self, board: &B, plies: u32, alpha: Score, mut beta: Score) -> Option<MoveStats<B::Move>>
    {
        match self.stopper.try_recv() {
            Ok(_) => return None,
            Err(e) => {
                match e {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => panic!("stopper channel disconnected"),
                }
            }
        }

        let moves = board.gen_enemy_moves();

        /* Fail state if you can't move */
        if moves.len() == 0
        {
            return Some(MoveStats
            {
                mv: None,
                /* If enemy can't move, we win. */
                score: Score::Win,
                turns: 0,
                nodes_visited: 0,
            });
        }

        /* If you cannot proceed further */
        if plies == 0 || board.is_game_over()
        {
            return Some(MoveStats
            {
                mv: None,
                score: board.score(),
                turns: 0,
                nodes_visited: 0,
            });
        }

        let mut best = MoveStats {
            mv: None,
            /* Technically doesn't matter, but for consistancy's sake */
            score: Score::Win,
            turns: 0,
            nodes_visited: 0,
        };

        if let Some(precomputed_move) = self.enemy_ttable.get(board, plies)
        {
            return Some(precomputed_move);
        }

        for mv in moves
        {
            /* Make a clone of the board so we don't break this one */
            let mut board_clone = board.clone();
            board_clone.do_move(&mv);

            /* Find ally's best move */
            let ally_move = match self.max(&board_clone, plies - 1, alpha, beta) {
                Some(mv) => mv,
                None => return None,
            };

            if best.mv.is_none() || ally_move.enemy_cmp(&best) == Ordering::Less
            {
                best.mv = Some(mv);
                best.score = ally_move.score;
                best.turns = ally_move.turns + 1;
            }

            best.nodes_visited += ally_move.nodes_visited + 1;

            /* Set β and break on β ≤ α */
            if best.score < beta
            {
                beta = ally_move.score;
            }
            if alpha >= beta
            {
                break;
            }
        }

        self.enemy_ttable.insert(board.clone(), best.clone(), plies);

        Some(best)
    }
}

#[derive(Clone)]
struct MinimaxArgs<B>
    where B: Board + Eq + Hash
{
    board: B,
    team: Team,
}

pub struct NonBlockingMinimax<B>
    where B: Board + Eq + Hash
{
    sender: Sender<MinimaxArgs<B>>,
    stopper: Sender<()>,
    receiver: Receiver<Option<MoveStats<B::Move>>>,
}

impl<B> NonBlockingMinimax<B>
    where B: Board + Eq + Hash + Send + 'static,
          B::Move: Send,
{
    pub fn new(ttable_size: usize) -> NonBlockingMinimax<B>
    {
        let (main_sender, thread_receiver) = mpsc::channel();
        let (stop_sender, stop_receiver) = mpsc::channel();
        let (thread_sender, main_receiver) = mpsc::channel();

        let mut minimax = Minimax::new(ttable_size, stop_receiver);

        thread::spawn(move || {
            loop {
                let args: MinimaxArgs<B> = thread_receiver.recv().unwrap();
                let mut best = None;
                let mut plies = 1;
                loop {
                    match minimax.minimax(&args.board, args.team, plies) {
                        Some(result) => {
                            best = Some(result);
                            println!("Got to ply {}.", plies);
                            plies += 1;
                        }
                        None => break,
                    }
                }
                thread_sender.send(best);
            }
        });

        NonBlockingMinimax {
            sender: main_sender,
            stopper: stop_sender,
            receiver: main_receiver,
        }
    }

    pub fn start_iterative_deepening(&self, board: &B, team: Team) {
        self.sender.send(MinimaxArgs{ board: board.clone(), team: team });
    }

    pub fn stop_iterative_deepening(&self) -> Option<MoveStats<B::Move>> {
        self.stopper.send(());
        self.receiver.recv().unwrap()
    }
}

//#[cfg(test)]
//mod tests
//{
//    use super::{Team, Score, Minimax, MoveStats};
//    use board::Board;
//
//    #[derive(Clone,PartialEq,Eq,Debug)]
//    struct SimpleMove(usize);
//    #[derive(Clone,PartialEq,Eq,Hash,Debug)]
//    enum SimpleBoard
//    {
//        Node(Vec<SimpleBoard>),
//        Leaf(Score),
//    }
//
//    /* These aren't really efficient, but they
//     * work for testing.
//     */
//    impl Board for SimpleBoard
//    {
//        type Move = SimpleMove;
//
//        fn gen_ally_moves(&self) -> Vec<Self::Move>
//        {
//            match *self
//            {
//                SimpleBoard::Node(ref v) =>
//                    (0..v.len()).map(|u| SimpleMove(u)).collect(),
//                SimpleBoard::Leaf(_) =>
//                    /* Trick the algorithm into thinking
//                     * we still have moves left
//                     */
//                    vec![SimpleMove(1)],
//            }
//        }
//
//        fn gen_enemy_moves(&self) -> Vec<Self::Move>
//        {
//            match *self
//            {
//                SimpleBoard::Node(ref v) =>
//                    (0..v.len()).map(|u| SimpleMove(u)).collect(),
//                SimpleBoard::Leaf(_) =>
//                    /* Trick the algorithm into thinking
//                     * we still have moves left
//                     */
//                    vec![SimpleMove(1)],
//            }
//        }
//
//        fn do_move(&mut self, mv: &Self::Move)
//        {
//            let child = match *self
//            {
//                SimpleBoard::Node(ref v) =>
//                    Some(v[mv.0].clone()),
//                SimpleBoard::Leaf(_) =>
//                    None,
//            };
//
//            if let Some(c) = child
//            {
//                *self = c;
//            }
//        }
//
//        fn score(&self) -> Score
//        {
//            match *self
//            {
//                SimpleBoard::Node(_) =>
//                    unreachable!(),
//                SimpleBoard::Leaf(s) =>
//                    s,
//            }
//        }
//
//        fn is_game_over(&self) -> bool
//        {
//            match *self
//            {
//                SimpleBoard::Node(_) =>
//                    false,
//                SimpleBoard::Leaf(_) =>
//                    true,
//            }
//        }
//    }
//
//    #[test]
//    fn test_score_ord()
//    {
//        assert!(Score::Win > Score::Heuristic(0));
//        assert!(Score::Heuristic(0) > Score::Lose);
//        assert!(Score::Win > Score::Lose);
//        assert!(Score::Heuristic(100) > Score::Heuristic(0));
//        assert!(Score::Heuristic(0) > Score::Heuristic(-100));
//        assert!(Score::Win == Score::Win);
//        assert!(Score::Lose == Score::Lose);
//        assert!(Score::Heuristic(0) == Score::Heuristic(0));
//    }
//
//    #[test]
//    fn test_minimax()
//    {
//        let mut minimax = Minimax::new(100);
//        let game1 =
//        SimpleBoard::Node(vec![
//            SimpleBoard::Node(vec![
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(5)),
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(7)),
//                        SimpleBoard::Leaf(Score::Heuristic(4)),
//                        SimpleBoard::Leaf(Score::Heuristic(5)),
//                    ]),
//                ]),
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(3)),
//                    ]),
//                ]),
//            ]),
//            SimpleBoard::Node(vec![
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                        SimpleBoard::Leaf(Score::Heuristic(9)),
//                    ]),
//                ]),
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(7)),
//                    ]),
//                ]),
//            ]),
//            SimpleBoard::Node(vec![
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(5)),
//                    ]),
//                ]),
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(9)),
//                        SimpleBoard::Leaf(Score::Heuristic(8)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                    ]),
//                ]),
//            ]),
//        ]);
//        let game2 =
//        SimpleBoard::Node(vec![
//            SimpleBoard::Node(vec![
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(12)),
//                        SimpleBoard::Leaf(Score::Heuristic(10)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(-18)),
//                        SimpleBoard::Leaf(Score::Heuristic(-7)),
//                    ]),
//                ]),
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                        SimpleBoard::Leaf(Score::Heuristic(-17)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(-3)),
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                    ]),
//                ]),
//            ]),
//            SimpleBoard::Node(vec![
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(-19)),
//                        SimpleBoard::Leaf(Score::Heuristic(-16)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(-4)),
//                        SimpleBoard::Leaf(Score::Heuristic(-6)),
//                    ]),
//                ]),
//                SimpleBoard::Node(vec![
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(-1)),
//                        SimpleBoard::Leaf(Score::Heuristic(6)),
//                    ]),
//                    SimpleBoard::Node(vec![
//                        SimpleBoard::Leaf(Score::Heuristic(9)),
//                        SimpleBoard::Leaf(Score::Heuristic(15)),
//                    ]),
//                ]),
//            ]),
//        ]);
//
//        println!();
//        println!("Game 1");
//        assert_eq!(game1.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1), SimpleMove(2)]);
//        let move_stats1 = minimax.minimax(&game1, Team::Ally, 4);
//        let optimal_move1 = MoveStats {
//            mv: Some(SimpleMove(1)),
//            score: Score::Heuristic(6),
//            turns: 4,
//            nodes_visited: 25,
//        };
//        assert_eq!(move_stats1, optimal_move1);
//
//        println!();
//        println!("Game 2");
//        assert_eq!(game2.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1)]);
//        let move_stats2 = minimax.minimax(&game2, Team::Ally, 4);
//        let optimal_move2 = MoveStats {
//            mv: Some(SimpleMove(0)),
//            score: Score::Heuristic(-3),
//            turns: 4,
//            nodes_visited: 21,
//        };
//        assert_eq!(move_stats2, optimal_move2);
//
//        println!();
//        println!("Testing caching");
//        assert_eq!(game2.gen_ally_moves(), vec![SimpleMove(0), SimpleMove(1)]);
//        let move_stats2 = minimax.minimax(&game2, Team::Ally, 4);
//        let optimal_move2 = MoveStats {
//            mv: Some(SimpleMove(0)),
//            score: Score::Heuristic(-3),
//            turns: 4,
//            nodes_visited: 21,
//        };
//        assert_eq!(move_stats2, optimal_move2);
//    }
//}
