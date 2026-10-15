//! How a console's shelf is ordered, and the sections that order falls
//! into (the aisle a game would stand in): all from the details LaunchBox
//! gave each game, so a game without details goes last.

use std::cmp::Ordering;

use crate::details::Details;
use crate::rentals::Rental;
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
    /// How often each game has been played, the most first.
    Plays,
}

impl SortKey {
    pub const ALL: [SortKey; 6] = [
        SortKey::Name,
        SortKey::Genre,
        SortKey::Year,
        SortKey::Publisher,
        SortKey::Rating,
        SortKey::Plays,
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
            SortKey::Plays => "plays",
        }
    }

    pub fn parse(word: &str) -> Option<SortKey> {
        SortKey::ALL
            .into_iter()
            .find(|k| word.trim().eq_ignore_ascii_case(k.word()))
    }
}

/// The rating from which a game is a staff pick.
pub const STAFF_PICK: f32 = 4.5;

/// From how many plays a game is a top rental, and from how many a regular.
pub const FAVOURITE_PLAYS: u32 = 10;
const REGULAR_PLAYS: u32 = 3;

fn rating_of(d: Option<&Details>) -> Option<f32> {
    d.and_then(|d| d.rating).filter(|r| *r > 0.0)
}

fn text_key(text: Option<&str>) -> Option<String> {
    text.map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
}

/// What is known of a game to sort it by: its details and its plays.
type Known<'a> = (&'a Cart, Option<&'a Details>, Option<Rental>);

/// How two games compare under `key`, a game with nothing to compare last.
fn compare(key: SortKey, a: Known, b: Known) -> Ordering {
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
        // The most played first, and of those played as often, the one
        // played last.
        SortKey::Plays => {
            let plays = |r: Option<Rental>| r.map_or((0, 0), |r| (r.times, r.last));
            plays(b.2).cmp(&plays(a.2))
        }
    };
    by.then_with(|| sort_key(&a.0.stem).cmp(&sort_key(&b.0.stem)))
}

/// The order `carts` stand in under `key`: the index of the cart for each
/// place on the shelf. Equal games stay in name order, and the sort is
/// stable, so the same details always give the same shelf.
/// `plays` gives how often each game has been played.
pub fn sorted_order<'a>(
    carts: &[Cart],
    details: impl Fn(&Cart) -> Option<&'a Details>,
    plays: impl Fn(&Cart) -> Option<Rental>,
    key: SortKey,
) -> Vec<usize> {
    let known: Vec<Known> = carts
        .iter()
        .map(|c| (c, details(c), plays(c)))
        .collect();
    let mut order: Vec<usize> = (0..carts.len()).collect();
    order.sort_by(|a, b| compare(key, known[*a], known[*b]));
    order
}

fn section_of(key: SortKey, (cart, d, plays): Known, plain: bool) -> String {
    match key {
        SortKey::Name => match initial(&cart.stem) {
            '#' => "0\u{2013}9".to_string(),
            c => c.to_string(),
        },
        SortKey::Plays if plain => match plays.map_or(0, |r| r.times) {
            0 => "NEVER PLAYED",
            n if n >= FAVOURITE_PLAYS => "MOST PLAYED \u{2605}",
            n if n >= REGULAR_PLAYS => "PLAYED OFTEN",
            _ => "PLAYED ONCE OR TWICE",
        }
        .to_string(),
        SortKey::Plays => match plays.map_or(0, |r| r.times) {
            0 => "NEW RELEASES",
            n if n >= FAVOURITE_PLAYS => "TOP RENTALS \u{2605}",
            n if n >= REGULAR_PLAYS => "REGULARS",
            _ => "RENTED ONCE OR TWICE",
        }
        .to_string(),
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
            Some(r) if r >= STAFF_PICK && plain => "TOP RATED \u{2605}",
            Some(r) if r >= STAFF_PICK => "STAFF PICKS \u{2605}",
            Some(r) if r >= 4.0 => "\u{2605}\u{2605}\u{2605}\u{2605}",
            Some(r) if r >= 3.0 => "\u{2605}\u{2605}\u{2605}",
            Some(_) => "\u{2605}\u{2605} & UNDER",
            None => "UNRATED",
        }
        .to_string(),
    }
}

/// The section each cart is in under `key`, in the carts' order, in the
/// video store's words.
pub fn sections<'a>(
    carts: &[Cart],
    details: impl Fn(&Cart) -> Option<&'a Details>,
    plays: impl Fn(&Cart) -> Option<Rental>,
    key: SortKey,
) -> Vec<String> {
    sections_worded(carts, details, plays, key, false)
}

/// The sections, in plain words (MOST PLAYED rather than TOP RENTALS)
/// when `plain`, for the original theme.
pub fn sections_worded<'a>(
    carts: &[Cart],
    details: impl Fn(&Cart) -> Option<&'a Details>,
    plays: impl Fn(&Cart) -> Option<Rental>,
    key: SortKey,
    plain: bool,
) -> Vec<String> {
    carts
        .iter()
        .map(|c| section_of(key, (c, details(c), plays(c)), plain))
        .collect()
}
