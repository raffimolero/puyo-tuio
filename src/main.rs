// TODO: remove this when ready
#![allow(warnings)]

use std::{
    array::from_fn,
    collections::{HashSet, VecDeque},
    fmt::Display,
    iter::repeat_with,
    time::{Duration, Instant},
};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    X,
    R,
    Y,
    G,
    B,
    P,
}

impl Color {
    const COLORS: [Self; 5] = [Self::R, Self::Y, Self::G, Self::B, Self::P];

    fn rand(rng: &mut impl Rng) -> Self {
        *Self::COLORS.choose(rng).unwrap()
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Puyo(Color);

impl Puyo {
    fn rand(rng: &mut impl Rng) -> Self {
        Self(Color::rand(rng))
    }
}

impl Display for Puyo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tile(Option<Puyo>);

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(puyo) = self.0 {
            puyo.fmt(f)
        } else {
            write!(f, "   ")
        }
    }
}

#[derive(Debug)]
struct Grid([[Tile; 6]; 12]);

impl Grid {
    const WIDTH: usize = 6;
    const HEIGHT: usize = 12;

    fn new() -> Self {
        Self::default()
    }

    fn get(&self, Point { x, y }: Point) -> Option<Tile> {
        ((0..Self::WIDTH as i8).contains(&x) && (0..Self::HEIGHT as i8).contains(&y))
            .then(|| self.0[y as usize][x as usize])
    }

    /// returns the previous tile state at that position.
    /// none if out of bounds.
    fn set(&mut self, Point { x, y }: Point, tile: Tile) -> Result<Tile, ()> {
        if !(0..Self::WIDTH as i8).contains(&x) || !(0..Self::HEIGHT as i8).contains(&y) {
            return Err(());
        }
        Ok(std::mem::replace(&mut self.0[y as usize][x as usize], tile))
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self([[Tile(None); 6]; 12])
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in &self.0 {
            for tile in row {
                write!(f, "{tile}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Pair([Puyo; 2]);

impl Pair {
    fn rand(rng: &mut impl Rng) -> Self {
        Self(from_fn(|_| Puyo::rand(rng)))
    }
}

impl Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for puyo in self.0 {
            write!(f, "{puyo}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    U,
    R,
    D,
    L,
}

impl Direction {
    fn rotated_cw(self) -> Self {
        match self {
            Direction::U => Direction::R,
            Direction::R => Direction::D,
            Direction::D => Direction::L,
            Direction::L => Direction::U,
        }
    }

    fn rotated_cc(self) -> Self {
        match self {
            Direction::U => Direction::L,
            Direction::R => Direction::U,
            Direction::D => Direction::R,
            Direction::L => Direction::D,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: i8,
    y: i8,
}

impl Point {
    fn shifted(mut self, rot: Direction) -> Self {
        match rot {
            Direction::U => self.y -= 1,
            Direction::R => self.x += 1,
            Direction::D => self.y += 1,
            Direction::L => self.x -= 1,
        }
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct PairPosition {
    anchor: Point,
    shift: Direction,
}

impl Default for PairPosition {
    fn default() -> Self {
        Self {
            anchor: Point {
                // 3rd column
                x: 2,
                // top of the board
                y: 0,
            },
            // the second piece spawns above the playfield
            shift: Direction::U,
        }
    }
}

#[derive(Debug)]
struct Board {
    queue: Vec<Pair>,
    active_pair: PairPosition,
    grid: Grid,
}

impl Board {
    fn new(rng: &mut impl Rng, queue_length: usize) -> Self {
        let mut randomizer = repeat_with(|| Pair::rand(rng));
        let mut this = Self {
            queue: randomizer.take(queue_length).collect(),
            active_pair: PairPosition::default(),
            grid: Grid::new(),
        };
        this.draw_active_pair();
        this
    }

    /// returns whether the active puyo locked.
    fn fall(&mut self) -> bool {
        self.clear_active_pair();

        let PairPosition { anchor, shift } = self.active_pair;
        let shifted = anchor.shifted(Direction::D);

        let can_fall = [
            self.grid.get(shifted),
            self.grid.get(shifted.shifted(shift)),
        ] == [Some(Tile(None)); 2];

        if can_fall {
            self.active_pair.anchor = shifted;
        }

        self.draw_active_pair();
        can_fall
    }

    fn draw_active_pair(&mut self) {
        let p @ Pair([a, b]) = self.queue[0];
        dbg!(p);
        let PairPosition { anchor, shift } = self.active_pair;
        self.grid
            .set(anchor, Tile(Some(a)))
            .expect("primary must be in bounds");
        let _ = self.grid.set(anchor.shifted(shift), Tile(Some(b)));
    }

    fn clear_active_pair(&mut self) {
        let Pair([a, b]) = self.queue[0];
        let PairPosition { anchor, shift } = self.active_pair;
        self.grid
            .set(anchor, Tile(None))
            .expect("primary must be in bounds");
        let _ = self.grid.set(anchor.shifted(shift), Tile(None));
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QUEUE: ")?;
        for pair in &self.queue {
            write!(f, "{pair} | ")?;
        }
        writeln!(f)?;
        writeln!(f, "{}", self.grid)
    }
}

#[derive(Debug)]
struct GameState {
    tick_time: Duration,
    rng: ThreadRng,
    board: Board,
}

impl GameState {
    fn new(tick_time: Duration, queue_length: usize) -> Self {
        let mut rng = thread_rng();
        // let grid = Grid(from_fn(|_| from_fn(|_| Tile(Some(Puyo::rand(&mut rng))))));
        let board = Board::new(&mut rng, queue_length);
        Self {
            tick_time,
            rng,
            board,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(Duration::from_millis(250), 2)
    }
}

impl Game for GameState {
    fn key_down(&mut self, key: KeyCode) {
        println!("down: {key:?}");
        // todo!("keyboard input")
    }

    fn key_up(&mut self, key: KeyCode) {
        println!("up: {key:?}");
    }

    fn tick(&mut self) {
        self.board.fall();
    }

    fn tick_time(&self) -> Duration {
        self.tick_time
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.board)
    }
}

trait Game: Display {
    fn key_down(&mut self, key: KeyCode) {}
    fn key_up(&mut self, key: KeyCode) {}
    fn tick(&mut self) {}

    fn tick_time(&self) -> Duration;

    fn run(&mut self) -> crossterm::Result<()> {
        let mut next_tick = Instant::now();
        let mut held = HashSet::new();

        enable_raw_mode();
        loop {
            if poll(next_tick - Instant::now())? {
                let event = read()?;
                if let Event::Key(KeyEvent {
                    code,
                    modifiers: _,
                    kind,
                    state: _,
                }) = event
                {
                    if code == KeyCode::Char('q') {
                        break;
                    }

                    if kind == KeyEventKind::Press {
                        if held.insert(code) {
                            self.key_down(code);
                        }
                    } else {
                        held.remove(&code);
                        self.key_up(code);
                    }
                }
            } else {
                next_tick = Instant::now() + self.tick_time();
                self.tick();
            }
            println!("{self}");
        }

        disable_raw_mode();
        Ok(())
    }
}

fn main() {
    GameState::default().run();
}
