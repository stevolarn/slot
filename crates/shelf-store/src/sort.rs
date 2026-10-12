//! How a console's shelf is ordered, and the sections that order falls
//! into (the aisle a game would stand in): all from the details LaunchBox
//! gave each game, so a game without details goes last.

use std::cmp::Ordering;

use crate::details::Details;
use crate::scan::{initial, sort_key, Cart};

/// What the shelf is ordered by.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum SortKey {
    #[default]
    Name,
    Genre,
    Year,
    Publisher,
    Rating,
}

impl SortKey {
    pub const ALL: [SortKey; 5] = [
        SortKey::Name,
        SortKey::Genre,
        SortKey::Year,
        SortKey::Publisher,
        SortKey::Rating,
    ];

    /// R2: the next order, round to the first.
    pub fn next(self) -> SortKey {
        let at = SortKey::ALL.iter().position(|k| *k == self).unwrap_or(0);
        SortKey::ALL[(at + 1) % SortKey::ALL.len()]
    }

    pub fn prev(self) -> SortKey {
        let at = SortKey::ALL.iter().position(|k| *k == self).unwrap_or(0);
        SortKey::ALL[(at + SortKey::ALL.len() - 1) % SortKey::ALL.len()]
    }

    /// The word the order is saved and shown as.
    pub fn word(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Genre => "genre",
            SortKey::Year => "year",
            SortKey::Publisher => "publisher",
            SortKey::Rating => "rating",
        }
    }

    pub fn parse(word: &str) -> Option<SortKey> {
        SortKey::ALL
            .into_iter()
            .find(|k| word.trim().eq_ignore_ascii_case(k.word()))
    }
}

/// Which way the shelves are shown.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum DisplayMode {
    /// Carts in the console's slot.
    #[default]
    Carts,
    /// Boxes standing face-out on a video store's shelf.
    Store,
    /// Box fronts in the dark, one large in the middle, as the carts are.
    Boxes,
}

impl DisplayMode {
    pub fn word(self) -> &'static str {
        match self {
            DisplayMode::Carts => "carts",
            DisplayMode::Store => "store",
            DisplayMode::Boxes => "boxes",
        }
    }

    pub fn parse(word: &str) -> Option<DisplayMode> {
        DisplayMode::ALL
            .into_iter()
            .find(|m| word.trim().eq_ignore_ascii_case(m.word()))
    }

    /// The displays in the order Display steps through them.
    pub const ALL: [DisplayMode; 3] = [DisplayMode::Store, DisplayMode::Carts, DisplayMode::Boxes];

    pub fn next(self) -> DisplayMode {
        Self::step(self, 1)
    }

    pub fn prev(self) -> DisplayMode {
        Self::step(self, Self::ALL.len() - 1)
    }

    fn step(self, by: usize) -> DisplayMode {
        let at = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(at + by) % Self::ALL.len()]
    }
}

/// The rating from which a game is a staff pick.
pub const STAFF_PICK: f32 = 4.5;

/// A shelf this small is sectioned by single letters, not chunks of them.
const SMALL_SHELF: usize = 24;

fn rating_of(d: Option<&Details>) -> Option<f32> {
    d.and_then(|d| d.rating).filter(|r| *r > 0.0)
}

fn text_key(text: Option<&str>) -> Option<String> {
    text.map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
}

/// How two games compare under `key`, a game with nothing to compare last.
fn compare(key: SortKey, a: (&Cart, Option<&Details>), b: (&Cart, Option<&Details>)) -> Ordering {
    fn present<T: PartialOrd>(a: Option<T>, b: Option<T>, flip: bool) -> Ordering {
        match (a, b) {
            (Some(a), Some(b)) => {
                let o = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if flip {
                    o.reverse()
                } else {
                    o
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
    let by = match key {
        SortKey::Name => Ordering::Equal,
        SortKey::Genre => present(
            text_key(a.1.and_then(|d| d.genres.first()).map(String::as_str)),
            text_key(b.1.and_then(|d| d.genres.first()).map(String::as_str)),
            false,
        ),
        SortKey::Year => present(a.1.and_then(|d| d.year), b.1.and_then(|d| d.year), false),
        SortKey::Publisher => present(
            text_key(a.1.and_then(|d| d.publisher.as_deref())),
            text_key(b.1.and_then(|d| d.publisher.as_deref())),
            false,
        ),
        SortKey::Rating => present(rating_of(a.1), rating_of(b.1), true),
    };
    by.then_with(|| sort_key(&a.0.stem).cmp(&sort_key(&b.0.stem)))
}

/// The order `carts` stand in under `key`: the index of the cart for each
/// place on the shelf. Equal games stay in name order, and the sort is
/// stable, so the same details always give the same shelf.
pub fn sorted_order<'a>(
    carts: &[Cart],
    details: impl Fn(&Cart) -> Option<&'a Details>,
    key: SortKey,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..carts.len()).collect();
    order.sort_by(|a, b| {
        compare(
            key,
            (&carts[*a], details(&carts[*a])),
            (&carts[*b], details(&carts[*b])),
        )
    });
    order
}

/// Letters sectioned in threes, the last chunk taking the rest.
fn chunk_of(letter: char) -> &'static str {
    match letter {
        'A'..='C' => "A\u{2013}C",
        'D'..='F' => "D\u{2013}F",
        'G'..='I' => "G\u{2013}I",
        'J'..='L' => "J\u{2013}L",
        'M'..='O' => "M\u{2013}O",
        'P'..='R' => "P\u{2013}R",
        'S'..='U' => "S\u{2013}U",
        _ => "V\u{2013}Z",
    }
}

fn section_of(key: SortKey, cart: &Cart, d: Option<&Details>, small: bool) -> String {
    match key {
        SortKey::Name => match initial(&cart.stem) {
            '#' => "0\u{2013}9".to_string(),
            c if small => c.to_string(),
            c => chunk_of(c).to_string(),
        },
        SortKey::Genre => d
            .and_then(|d| d.genres.first())
            .map_or("OTHER".to_string(), |g| g.trim().to_uppercase()),
        SortKey::Year => d
            .and_then(|d| d.year)
            .map_or("YEAR UNKNOWN".to_string(), |y| y.to_string()),
        SortKey::Publisher => d
            .and_then(|d| d.publisher.as_deref())
            .map_or("PUBLISHER UNKNOWN".to_string(), |p| p.trim().to_uppercase()),
        SortKey::Rating => match rating_of(d) {
            Some(r) if r >= STAFF_PICK => "STAFF PICKS \u{2605}",
            Some(r) if r >= 4.0 => "\u{2605}\u{2605}\u{2605}\u{2605}",
            Some(r) if r >= 3.0 => "\u{2605}\u{2605}\u{2605}",
            Some(_) => "\u{2605}\u{2605} & UNDER",
            None => "UNRATED",
        }
        .to_string(),
    }
}

/// The section each cart is in under `key`, in the carts' order.
pub fn sections<'a>(
    carts: &[Cart],
    details: impl Fn(&Cart) -> Option<&'a Details>,
    key: SortKey,
) -> Vec<String> {
    let small = carts.len() <= SMALL_SHELF;
    carts
        .iter()
        .map(|c| section_of(key, c, details(c), small))
        .collect()
}
