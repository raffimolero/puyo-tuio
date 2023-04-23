// TODO: remove this when ready
// #![allow(warnings)]

use std::{
    array::from_fn,
    collections::{HashSet, VecDeque},
    fmt::Display,
    io::stdout,
    iter::repeat_with,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
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

impl Tile {
    fn is_free(self) -> bool {
        self.0.is_none()
    }

    fn is_occupied(self) -> bool {
        self.0.is_some()
    }
}

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
struct Grid([[Tile; Self::WIDTH]; Self::HEIGHT]);

impl Grid {
    const WIDTH: usize = 6;
    const HEIGHT: usize = 12;

    fn new() -> Self {
        Self::default()
    }

    // /// returns the previous tile state at that position.
    // /// none if out of bounds.
    // fn set(&mut self, Point { x, y }: Point, tile: Tile) -> Result<Tile, ()> {
    //     if !(0..Self::WIDTH as i8).contains(&x) || !(0..Self::HEIGHT as i8).contains(&y) {
    //         return Err(());
    //     }
    //     Ok(std::mem::replace(&mut self.0[y as usize][x as usize], tile))
    // }

    /// checks if a point is in bounds.
    fn point_in_bounds(Point { x, y }: Point) -> bool {
        (0..Self::WIDTH as i8).contains(&x) && (0..Self::HEIGHT as i8).contains(&y)
    }

    fn get(&self, p @ Point { x, y }: Point) -> Option<Tile> {
        Self::point_in_bounds(p).then(|| self.0[y as usize][x as usize])
    }

    fn get_mut(&mut self, p @ Point { x, y }: Point) -> Option<&mut Tile> {
        Self::point_in_bounds(p).then(|| &mut self.0[y as usize][x as usize])
    }

    /// returns true if spot is in bounds and unoccupied.
    fn is_free(&self, p: Point) -> bool {
        self.get(p) == Some(Tile(None))
    }

    /// returns true if spot is either out of bounds or if a puyo occupies that slot.
    fn is_occupied(&self, p: Point) -> bool {
        !self.is_free(p)
    }

    /// returns true if the space was in bounds and occupied.
    #[must_use]
    fn try_remove(&mut self, p: Point) -> bool {
        let Some(tile) = self.get_mut(p) else {
            return false;
        };
        if tile.is_occupied() {
            tile.0 = None;
            true
        } else {
            false
        }
    }

    /// attempts to put a puyo in a specified tile.
    /// returns Ok if the tile was successfully set.
    #[must_use]
    fn try_place(&mut self, p: Point, puyo: Puyo) -> bool {
        let Some(tile) = self.get_mut(p) else {
            return false;
        };
        if tile.is_free() {
            tile.0 = Some(puyo);
            true
        } else {
            false
        }
    }

    #[must_use]
    fn try_shift(&mut self, bottom: Point) -> bool {
        if self.is_free(bottom.shifted(Direction::D)) {
            let col = bottom.x as usize;
            for y in (0..=bottom.y as usize).rev() {
                self.0[y + 1][col] = self.0[y][col];
            }
            self.0[0][col] = Tile(None);
            true
        } else {
            false
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self([[Tile(None); Self::WIDTH]; Self::HEIGHT])
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "+{}+", "-".repeat(Self::WIDTH * 3))?;
        for row in &self.0 {
            write!(f, "|")?;
            for tile in row {
                write!(f, "{tile}")?;
            }
            writeln!(f, "|")?;
        }
        write!(f, "+{}+", "-".repeat(Self::WIDTH * 3))?;
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

    fn rotated_180(self) -> Self {
        match self {
            Direction::U => Direction::D,
            Direction::R => Direction::L,
            Direction::D => Direction::U,
            Direction::L => Direction::R,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: i8,
    y: i8,
}

impl Point {
    fn shifted(mut self, shift: Direction) -> Self {
        match shift {
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

impl PairPosition {
    fn pair(&self) -> Point {
        self.anchor.shifted(self.shift)
    }

    fn rotate_cw(&mut self) {
        self.shift = self.shift.rotated_cw();
    }

    fn rotate_cc(&mut self) {
        self.shift = self.shift.rotated_cc();
    }

    fn kickback(&mut self) {
        self.anchor = self.anchor.shifted(self.shift.rotated_180())
    }
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

    fn draw_active_pair(&mut self) {
        let Pair([a, b]) = self.queue[0];
        let PairPosition { anchor, shift } = self.active_pair;
        assert!(self.grid.try_place(anchor, a), "primary must be in bounds");
        let _ = self.grid.try_place(anchor.shifted(shift), b);
    }

    fn clear_active_pair(&mut self) {
        let PairPosition { anchor, shift } = self.active_pair;
        assert!(self.grid.try_remove(anchor), "primary must be in bounds");
        let _ = self.grid.try_remove(anchor.shifted(shift));
    }

    /// returns true if the puyo fell properly.
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

    /// attempts to rotate the puyo clockwise.
    /// returns false if the puyo was unable to rotate.
    fn rotate_cw(&mut self) -> bool {
        let mut p = self.active_pair;

        p.rotate_cw();
        if self.grid.is_occupied(p.pair()) {
            p.kickback();
            if self.grid.is_occupied(p.anchor) {
                return false;
            }
        }

        self.clear_active_pair();
        self.active_pair = p;
        self.draw_active_pair();
        true
    }

    /// modifies the combo given, and returns true if the simulation is finished.
    fn simulate(&mut self, combo: &mut Combo) -> bool {
        dbg!(&combo.falling);
        for (x, y) in combo.falling.iter_mut().enumerate() {
            if *y == -1 {
                continue;
            }
            debug_assert!(y.is_positive());

            let bottom = Point { x: x as i8, y: *y };

            #[cfg(debug_assertions)]
            let Some(Tile(Some(_puyo))) = self
                .grid
                .get(bottom)
            else {
                panic!("bottom puyo must be occupied. {bottom:?}");
            };

            if self.grid.try_shift(bottom) {
                dbg!();
                *y += 1;
            } else {
                todo!("land this puyo.");
            }

            todo!(
                "
                    figure out how to:
                        1. land a puyo
                        2. shift the bottom location to the next falling puyo
                        3. wait for puyo at the bottom to settle
                        4. pop puyos
                        5. expand the falling locations"
            );
        }
        todo!()
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QUEUE: ")?;
        for pair in &self.queue {
            write!(f, "{pair} | ")?;
        }
        writeln!(f)?;
        write!(f, "{}", self.grid)
    }
}

#[derive(Debug)]
struct Combo {
    /// the length of the combo
    len: u8,
    /// the accumulated score over the whole combo
    score: u16,
    /// tracks which columns have falling puyos, and how high up the bottommost affected puyo is.
    /// if column is equal to height, puyos aren't falling.
    falling: [i8; Grid::WIDTH],
}

impl Combo {
    fn new(pair_position: PairPosition) -> Self {
        let mut falling = [-1; Grid::WIDTH];

        let PairPosition { anchor, shift } = pair_position;
        falling[anchor.x as usize] = anchor.y;

        let shifted = anchor.shifted(shift);
        let col = &mut falling[shifted.x as usize];
        *col = (*col).max(shifted.y);

        Self {
            len: 0,
            score: 0,
            falling,
        }
    }

    fn is_still_falling(&self) -> bool {
        self.falling.iter().any(|&y| y != -1)
    }
}

impl Display for Combo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            len,
            score,
            falling: _,
        } = self;
        write!(f, "Combo: {len}")?;
        if *score > 0 {
            write!(f, "\nScore: {score}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct GameState {
    board: Board,
    combo: Option<Combo>,
    tick_time: Duration,
    rng: ThreadRng,
}

impl GameState {
    fn new(tick_time: Duration, queue_length: usize) -> Self {
        let mut rng = thread_rng();
        Self {
            board: Board::new(&mut rng, queue_length),
            combo: None,
            tick_time,
            rng,
        }
    }

    fn controllable(&self) -> bool {
        self.combo.is_none()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(Duration::from_millis(500), 2)
    }
}

impl Game for GameState {
    fn key_down(&mut self, key: KeyCode) {
        if !self.controllable() {
            return;
        }

        self.board.rotate_cw();
        println!("down: {key:?}");
        // todo!("keyboard input")
    }

    fn key_up(&mut self, key: KeyCode) {
        if !self.controllable() {
            return;
        }
        println!("up: {key:?}");
    }

    fn tick(&mut self) {
        if let Some(combo) = &mut self.combo {
            if !self.board.simulate(combo) {
                self.combo = None;
            }
        } else {
            if !self.board.fall() {
                self.combo = Some(Combo::new(self.board.active_pair));
            }
        }
    }

    fn tick_time(&self) -> Duration {
        self.tick_time * if self.controllable() { 1 } else { 2 }
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.board)?;
        if let Some(combo) = &self.combo {
            write!(f, "\n{combo}")?;
        }
        Ok(())
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
            execute!(
                stdout(),
                // Clear(ClearType::All),
                // MoveTo(0, 0),
                Print(&self)
            );
        }

        disable_raw_mode();
        Ok(())
    }
}

fn main() {
    GameState::default().run();
}
