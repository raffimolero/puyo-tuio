// TODO: remove this when ready
// #![allow(warnings)]

use std::{
    array::from_fn,
    collections::HashSet,
    fmt::Display,
    io::stdout,
    iter::repeat_with,
    mem::replace,
    ops::{Add, AddAssign},
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

    /// returns None if out of bounds, and Some containing the original tile if in bounds.
    fn try_remove(&mut self, p: Point) -> Option<Tile> {
        Some(Tile(self.get_mut(p)?.0.take()))
    }

    /// attempts to put a puyo in a specified tile.
    /// returns true if the tile was successfully set.
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

    /// returns true if the point had a puyo and was not blocked.
    fn try_fall(&mut self, point: Point) -> bool {
        let slot = point.shifted(Direction::D);
        if self.is_occupied(slot) {
            return false;
        }
        let Some(Tile(Some(puyo))) = self.try_remove(point) else {
            return false;
        };
        assert!(self.try_place(slot, puyo));
        true
    }

    fn pop(&mut self, mut pop: impl FnMut(usize)) {
        let mut checked = [[false; Self::WIDTH]; Self::HEIGHT];
        let mut queue = vec![];
        let mut chain = vec![];
        for y in 0..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                let Some(puyo) = self.0[y][x].0 else {
                    continue;
                };
                queue.push((x, y));
                while let Some(p @ (x, y)) = queue.pop() {
                    if self.0[y][x].0 != Some(puyo) {
                        continue;
                    };
                    if replace(&mut checked[y][x], true) {
                        continue;
                    }
                    chain.push(p);
                    if x > 0 {
                        queue.push((x - 1, y));
                    }
                    if x + 1 < Self::WIDTH {
                        queue.push((x + 1, y));
                    }
                    if y > 0 {
                        queue.push((x, y - 1));
                    }
                    if y + 1 < Self::HEIGHT {
                        queue.push((x, y + 1));
                    }
                }
                if chain.len() >= 4 {
                    pop(chain.len());
                    for (x, y) in chain.iter().copied() {
                        self.0[y][x] = Tile(None);
                    }
                }
                chain.clear();
            }
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
enum Rotation {
    N,
    CW,
    U,
    CC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    U,
    R,
    D,
    L,
}

impl Add<Rotation> for Direction {
    type Output = Self;

    fn add(self, rhs: Rotation) -> Self::Output {
        match (self as u8 + rhs as u8) % 4 {
            0 => Self::U,
            1 => Self::R,
            2 => Self::D,
            3 => Self::L,
            _ => unreachable!(),
        }
    }
}

impl AddAssign<Rotation> for Direction {
    fn add_assign(&mut self, rhs: Rotation) {
        *self = *self + rhs;
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

    fn rotate(&mut self, rotation: Rotation) {
        self.shift += rotation;
    }

    fn kickback(&mut self) {
        self.anchor = self.anchor.shifted(self.shift + Rotation::U)
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
    active_pair: Pair,
    queue: Vec<Pair>,
    pair_position: PairPosition,
    grid: Grid,
}

impl Board {
    fn new(rng: &mut impl Rng, queue_length: usize) -> Self {
        let mut randomizer = repeat_with(|| Pair::rand(rng));
        let mut this = Self {
            active_pair: randomizer.next().unwrap(),
            queue: randomizer.take(queue_length).collect(),
            pair_position: PairPosition::default(),
            grid: Grid::new(),
        };
        this.draw_active_pair();
        this
    }

    fn draw_active_pair(&mut self) {
        let Pair([a, b]) = self.active_pair;
        let PairPosition { anchor, shift } = self.pair_position;
        assert!(self.grid.try_place(anchor, a), "primary must be in bounds");
        let _ = self.grid.try_place(anchor.shifted(shift), b);
    }

    fn clear_active_pair(&mut self) {
        let PairPosition { anchor, shift } = self.pair_position;
        self.grid
            .try_remove(anchor)
            .expect("primary must be in bounds");
        let _ = self.grid.try_remove(anchor.shifted(shift));
    }

    /// attempts to spawn the next puyo pair.
    /// returns false if it was blocked.
    #[must_use]
    fn try_spawn_next_pair(&mut self, rng: &mut impl Rng) -> bool {
        self.pair_position = PairPosition::default();
        if self.grid.is_occupied(self.pair_position.anchor) {
            return false;
        }

        self.active_pair = self.queue.remove(0);
        self.queue.push(Pair::rand(rng));
        true
    }

    /// returns true if the puyo fell and wasn't blocked.
    fn shift(&mut self, direction: Direction) -> bool {
        let PairPosition { anchor, shift } = self.pair_position;
        let shifted = anchor.shifted(direction);

        self.clear_active_pair();
        let can_fall = self.grid.is_free(shifted) && self.grid.is_free(shifted.shifted(shift));

        if can_fall {
            self.pair_position.anchor = shifted;
        }

        self.draw_active_pair();
        can_fall
    }

    /// attempts to rotate the puyo clockwise.
    /// returns false if the puyo was unable to rotate.
    fn rotate(&mut self, rotation: Rotation) -> bool {
        let mut p = self.pair_position;

        p.rotate(rotation);
        if self.grid.is_occupied(p.pair()) {
            p.kickback();
            if self.grid.is_occupied(p.anchor) {
                return false;
            }
        }

        self.clear_active_pair();
        self.pair_position = p;
        self.draw_active_pair();
        true
    }

    /// makes all floating puyos fall.
    /// returns false if none fell.
    fn gravity(&mut self) -> bool {
        let mut fell = false;
        for y in (0..Grid::HEIGHT as i8).rev() {
            for x in 0..Grid::WIDTH as i8 {
                fell |= self.grid.try_fall(Point { x, y });
            }
        }
        fell
    }

    /// returns whether any puyos were popped at all.
    fn pop(&mut self, combo: &mut Combo) -> bool {
        let mut popped = false;
        self.grid.pop(|count| {
            popped = true;
            combo.pop(count as u32);
        });
        if popped {
            combo.len += 1;
        }
        popped
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
    len: u32,
    /// the accumulated score over the whole combo
    score: u32,
}

impl Combo {
    fn new() -> Self {
        Self { len: 0, score: 0 }
    }

    fn pop(&mut self, count: u32) {
        self.score += self.len * count;
    }
}

impl Display for Combo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { len, score } = self;
        write!(f, "Combo: {len}")?;
        if *score > 0 {
            write!(f, "\nScore: {score}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct GameState {
    dead: bool,
    board: Board,
    score: u32,
    combo: Option<Combo>,
    tick_time: Duration,
    rng: ThreadRng,
}

impl GameState {
    fn new(tick_time: Duration, queue_length: usize) -> Self {
        let mut rng = thread_rng();
        Self {
            dead: false,
            board: Board::new(&mut rng, queue_length),
            score: 0,
            combo: None,
            tick_time,
            rng,
        }
    }

    fn controllable(&self) -> bool {
        self.combo.is_none()
    }

    fn begin_combo(&mut self) {
        self.combo = Some(Combo::new());
    }

    /// ends the combo and spawns the next puyo pair.
    /// may end the game.
    fn end_combo(&mut self) {
        self.score += self
            .combo
            .take()
            .expect("attempted to end nonexistent combo")
            .score;
        self.dead = !self.board.try_spawn_next_pair(&mut self.rng);
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new(Duration::from_millis(250), 2)
    }
}

impl Game for GameState {
    fn key_down(&mut self, key: KeyCode) {
        if !self.controllable() {
            return;
        }

        match key {
            KeyCode::Char('j') => self.board.shift(Direction::L),
            KeyCode::Char('l') => self.board.shift(Direction::R),
            KeyCode::Char('k') => self.board.shift(Direction::D),
            KeyCode::Char('s') => self.board.rotate(Rotation::CC),
            KeyCode::Char('f') => self.board.rotate(Rotation::CW),
            _ => false, // TODO: do something with the return values lmao
        };
    }

    fn key_up(&mut self, key: KeyCode) {
        if !self.controllable() {
            return;
        }
        println!("up: {key:?}");
    }

    fn tick(&mut self) -> Duration {
        if self.dead {
            return self.tick_time * 5;
        }
        if let Some(combo) = &mut self.combo {
            if !self.board.gravity() && !self.board.pop(combo) {
                self.end_combo();
            }
        } else {
            if !self.board.shift(Direction::D) {
                self.begin_combo();
            }
        }
        self.tick_time / if self.controllable() { 1 } else { 2 }
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.board)?;
        if let Some(combo) = &self.combo {
            write!(f, "\n{combo}")?;
        }
        if self.dead {
            write!(f, "\nGAME OVER!")?;
        }
        Ok(())
    }
}

trait Game: Display {
    /// called when the user presses a key.
    fn key_down(&mut self, key: KeyCode) {
        println!("Pressed {key:?}");
    }

    /// called when the user releases a key.
    /// hopefully.
    fn key_up(&mut self, key: KeyCode) {
        println!("Released {key:?}");
    }

    /// returns how long the game should wait before the next frame.
    fn tick(&mut self) -> Duration {
        println!("Tick!");
        Duration::from_secs(1)
    }

    /// runs the game.
    fn run(&mut self) -> crossterm::Result<()> {
        let mut next_tick = Instant::now();
        let mut held = HashSet::new();

        enable_raw_mode()?;
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
                next_tick = Instant::now() + self.tick();
            }
            execute!(
                stdout(),
                //
                Clear(ClearType::All),
                MoveTo(0, 0),
                Print(&self),
            )?;
        }
        disable_raw_mode()?;

        Ok(())
    }
}

fn main() {
    GameState::default().run().unwrap();
}
