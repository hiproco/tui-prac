use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    terminal, Command, ExecutableCommand, QueueableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{canvas::Map, *},
};
use std::{
    any::TypeId,
    borrow::{BorrowMut, Cow},
    cell::RefCell,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::{self, prelude::*, Error, Stdout},
    iter::FromIterator,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Not},
    path::{Path, PathBuf},
};
use tui_prac::*;

use crate::card_widgets::Card;

// make a simple game
// turn base?
// units?
// refine more game idea
// keep simple
// as practice of cat-choco app
// card?
mod raw_mode {
    pub use crossterm::terminal::{disable_raw_mode as disable, enable_raw_mode as enable};
}

fn main() -> io::Result<()> {
    use ratatui::backend::Backend as Back;

    struct TerminalGuard<B: Back + ExecutableCommand> {
        terminal: Terminal<B>,
    }
    impl TerminalGuard<CrosstermBackend<Stdout>> {
        fn new() -> io::Result<Self> {
            let mut terminal = init_terminal()?;
            raw_mode::enable()?;
            terminal
                .backend_mut()
                .execute(terminal::EnterAlternateScreen)?;
            Ok(TerminalGuard { terminal })
        }
    }
    impl<B: Back + ExecutableCommand> Drop for TerminalGuard<B> {
        fn drop(&mut self) {
            raw_mode::disable().expect("could not disable raw mode");
            self.terminal
                .backend_mut()
                .execute(terminal::LeaveAlternateScreen)
                .expect("could not leave alternate screen");
        }
    }
    let mut guard = TerminalGuard::new()?;
    gameloop(&mut guard.terminal)?;
    Ok(())
}

pub mod network;
pub mod game {
    use std::collections::VecDeque;

    #[derive(Debug, Default, Clone)]
    struct Card;
    #[derive(Debug, Default)]
    pub struct Player {
        hand: Vec<Card>,
    }
    impl<C> AsRef<C> for Player
    where
        Vec<Card>: AsRef<C>,
    {
        fn as_ref(&self) -> &C {
            self.hand.as_ref()
        }
    }

    impl Player {
        pub fn connect() {
            let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0))
                .expect("failed to connect");
            let laddr = socket.local_addr().expect("no local address");
            std::fs::write("log/port.log", format!("{}", laddr.port())).expect("failed to write");
        }
    }

    pub(crate) fn thegameloop() {
        let mut players = VecDeque::from([Player::default()]);
        let mut stack = vec![Card];
        loop {
            let Some(situation) = stack.pop() else {
                break;
            };
            let Some(player) = players.pop_front() else {
                break;
            };

            let pass = vote(players.make_contiguous());

            players.push_back(player);
        }
    }

    fn vote(players: &[Player]) -> bool {
        players
            .into_iter()
            .map(choose)
            .reduce(|a, b| a && b)
            .expect("no players to vote")
    }

    fn choose(player: &Player) -> bool {
        true
    }
}

fn gameloop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error> {
    let data = load_data(terminal)?;
    let mut selected_tab = 0;
    let mut gamestate = GameState::default();
    gamestate.changed = true;
    loop {
        gamestate.changed = true;
        std::mem::take(&mut gamestate.changed)
            .then_some(terminal.borrow_mut())
            .map(|t| t.draw(draw_fn(Selector::Tab(selected_tab))))
            .transpose()?;
        use crossterm::event::*;
        if poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press,
                    state,
                }) => match code {
                    KeyCode::Char('q') => {
                        break;
                    }
                    KeyCode::Char('z' | 'x' | 'c') => {}
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}

// note to myself
// guard type
//     a wrapper/new type with drop implement?
// new type / wrapper
// should at least implement Deref/Mut trait and From trait with inner?
// From as wrap / unwrap function for new type/ single wrapper types?
// just use extension trait if possible

fn load_data(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<String, io::Error> {
    let (s, r) = std::sync::mpsc::channel();
    let load = std::thread::spawn(move || -> io::Result<String> {
        let data = std::fs::read_to_string("data/cards.csv").unwrap();

        let mut logger = try_init_logger(20).expect("fail to initialize logger file");
        let mut lines = data.lines();
        let raw_header = lines.next().expect("missing header");
        let header = raw_header.split(',');
        let (items, events): (Vec<_>, Vec<_>) = lines
            .filter_map(|l| {
                let mut row = l
                    .split(',')
                    .zip(header.clone())
                    .filter_map(|(s, h)| (!h.is_empty()).then_some(s));
                let [iid, iname, iset, ien, iimage, inotes, o, eid, ename, eitems, eset, een, eimage, enotes] = row.by_ref()
                    .take(14).map(|s| s.to_string()).collect::<Vec<_>>().try_into().ok()?;
                let item = (iid, iname, iset, ien, iimage, inotes);
                let event = (eid, ename, eset, een, eimage, enotes);
                let [item, event] =
                    [item, event].map(|(id, name, set, en, image, notes)| CardAttribute {
                        id: id.parse().unwrap_or_default(),
                        name,
                        set: set.parse().unwrap_or_default(),
                        en,
                        image: image.is_empty().not().then(|| image.into()),
                        notes,
                    });
                let item = Item { attribute: item };
                let event = Event {
                    attribute: event,
                    items: eitems.parse().unwrap_or_default(),
                };
                Some(Schema {
                    item,
                    other: o.is_empty().not().then_some(o),
                    event,
                })
            })
            .map(|s| (s.item, s.event))
            .unzip();

        #[derive(Debug)]
        struct CardAttribute {
            id: u16,
            name: String,
            set: u8,
            en: String,
            image: Option<PathBuf>,
            notes: String,
        }
        #[derive(Debug)]
        struct Item {
            attribute: CardAttribute,
        }
        #[derive(Debug)]
        struct Event {
            attribute: CardAttribute,
            items: u8,
        }
        #[derive(Debug)]
        struct Schema {
            item: Item,
            other: Option<String>,
            event: Event,
        }

        for i in items.iter() {
            writeln!(logger, "[item card] {:?}", i)?;
        }
        for e in events.iter() {
            writeln!(logger, "[event card] {:?}", e)?;
        }

        let mut data_header = std::fs::File::create("log/header.txt")?;

        let mut buf = raw_header.as_bytes();
        while !buf.is_empty() {
            let amt = data_header.write(buf)?;
            s.send(amt as u16).map_err(|e| Error::other(e))?;
            writeln!(logger, "[written {} bytes]", amt);
            buf.consume(amt);
        }

        Ok(data)
    });
    let mut p = 0;
    let len = r.recv().expect("failed to get file length");
    while !load.is_finished() {
        terminal.draw(|f| {
            p = r.try_recv().unwrap_or(p);
            f.render_widget(
                Gauge::default()
                    .block(Block::default().borders(Borders::ALL))
                    .percent(((len - p) * 100) / len),
                f.size(),
            )
        })?;
    }
    load.join()
        .map_err(|_| io::Error::other("fail to load data"))?
}

fn try_init_logger(tries: i32) -> io::Result<io::BufWriter<File>> {
    (0..tries)
        .find_map(|t| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(format!("log/log{}.log", t))
                .ok()
        })
        .map(std::io::BufWriter::new)
        .ok_or(std::io::Error::other("cannot create logging file"))
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, io::Error> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    Ok(terminal)
}

fn draw_fn(selected: Selector) -> impl for<'a, 'b> FnOnce(&'a mut Frame<'b>) {
    const BORDERED: Block = Block::new().borders(Borders::ALL);
    use ratatui::layout::Constraint::*;
    let layouts = [
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // Max(1),
                Min(3),
                Max(5),
            ]),
        // Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints([Ratio(1, 3); 3]),
    ];
    move |frame: &mut Frame| {
        let buf = frame.buffer_mut();
        let binding = layouts[0].split(buf.area);
        let mut areas = binding.into_iter().copied();

        let selected = selected.selected_tab().unwrap_or(0);
        card_widgets::Card::new()
            .name("situation")
            .render(areas.next().unwrap(), buf);
        card_widgets::GeneralList::<Card>::new()
            .items(["item 1", "item  2", "item 3"].map(Card::from_name))
            .direction(Direction::Horizontal)
            .render(areas.next().unwrap(), buf);
    }
}

trait Apply<I, O>: FnMut(I) -> O + Sized {
    fn preapply(self, iter: impl IntoIterator<Item = I>) -> impl Iterator<Item = O> {
        iter.into_iter().map(self)
    }
    fn apply<C: FromIterator<O>>(self, iter: impl IntoIterator<Item = I>) -> C {
        self.preapply(iter).collect()
    }
}
impl<I, O, F: FnMut(I) -> O> Apply<I, O> for F {}

mod card_widgets {

    use ratatui::{layout::Constraint, prelude::*, widgets::*};

    #[derive(Debug, Default)]
    pub struct Card {
        name: String,
        description: String,
    }

    impl Card {
        pub fn from_name<S: Into<String>>(s: S) -> Self {
            Self::new().name(s)
        }

        pub fn new() -> Self {
            Self::default()
        }

        pub fn name<S: Into<String>>(self, name: S) -> Self {
            Self {
                name: name.into(),
                ..self
            }
        }
        pub fn description<S: Into<String>>(self, description: S) -> Self {
            Self {
                description: description.into(),
                ..self
            }
        }
    }
    impl Widget for Card {
        fn render(self, area: Rect, buf: &mut Buffer) {
            Paragraph::new(self.description)
                .block(Block::new().borders(Borders::ALL).title(self.name))
                .render(area, buf);
        }
    }

    #[derive(Debug)]
    pub struct GeneralList<W: Widget> {
        direction: Direction,
        items: Vec<W>,
    }
    impl<W: Widget> GeneralList<W> {
        pub fn new() -> Self {
            Self {
                direction: Default::default(),
                items: vec![],
            }
        }
        pub fn direction(self, direction: Direction) -> Self {
            Self { direction, ..self }
        }
        pub fn items(self, items: impl IntoIterator<Item = W>) -> Self {
            Self {
                items: items.into_iter().collect(),
                ..self
            }
        }
    }

    impl<W: Widget> Widget for GeneralList<W> {
        fn render(self, area: Rect, buf: &mut Buffer) {
            let len = self.items.len();
            for (area, item) in Layout::new(
                self.direction,
                Constraint::from_ratios(vec![(1, len as u32); len as usize]),
            )
            .split(area)
            .into_iter()
            .copied()
            .zip(self.items)
            {
                item.render(area, buf);
            }
        }
    }
}

#[derive(Debug, Default)]
struct GameState {
    changed: bool,
    states: GameData,
}
type GameData = ();
enum Selector {
    Tab(usize),
}
impl Selector {
    fn selected_tab(&self) -> Option<usize> {
        use Selector::*;
        match self {
            Tab(selected) => Some(*selected),
            _ => None,
        }
    }
}
struct LayoutTree {
    layout: Layout,
    child: Box<[LayoutTree]>,
}
trait IntoInner {
    type Inner;
    fn into_inner() {}
}
struct Log<T, F: FnMut(&T)>(T, RefCell<F>);
impl<T, F: FnMut(&T)> Log<T, F> {
    fn into_inner(mut self) -> T {
        self.1.get_mut()(&self.0);
        self.0
    }
}
impl<T, F: FnMut(&T)> Deref for Log<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.1.borrow_mut()(&self.0);
        &self.0
    }
}
impl<T, F: FnMut(&T)> DerefMut for Log<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.borrow_mut()(&self.0);
        &mut self.0
    }
}
trait Loggable: Debug + Sized {
    fn with_logger<F: FnMut(&Self)>(self, f: F) -> Log<Self, F> {
        Log(self, f.into())
    }
}
impl<T: Debug> Loggable for T {}

#[cfg(test)]
mod tests {
    #[test]
    fn cell_take() {
        let mut once = std::cell::OnceCell::new();
        let _ = once.set(1);
        let taken = once.take();
        dbg!(taken);
        let r = once.set(2);
        dbg!(r);
    }

    #[test]
    fn refs() {
        let a = &1;
        let b = &1;
        assert!(a == b);
    }
    use std::io::{self, BufRead};

    use crate::Loggable;

    #[test]
    fn bufread_bytes() -> io::Result<()> {
        let mut a = [0u8; 100].as_slice();
        a.fill_buf()?;
        Ok(())
    }

    #[test]
    fn take_byref() {
        let a = [0u8; 100];
        let mut iter = a.iter();
        let v = [0u8; 50];
        let first_take = iter.by_ref().zip(v).count();
        let v = [0u8; 25];
        let second_take = iter.by_ref().zip(v).count();
        let rest = iter.count();
        assert_eq!(first_take + second_take + rest, 100)
    }

    #[test]
    fn log() {
        let a = 0;
        let mut a = a.with_logger(|i| {
            dbg!(i);
        });
        *a += 1;
        let a = a.into_inner();
    }

    #[test]
    fn player_connect() {
        crate::game::Player::connect();
    }
}
