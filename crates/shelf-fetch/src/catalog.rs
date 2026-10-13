//! Reading LaunchBox's Metadata.xml as it streams in, keeping only what the
//! games on the shelves need: their details and the file names of their
//! box front, box back, cart photo, title screen and a gameplay
//! screenshot. The file lists every game's entries first, then the games'
//! other names, then their pictures, so the games are matched before the
//! pictures go by and only theirs are kept.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering};

use quick_xml::events::Event;
use shelf_store::Platform;

use crate::names::{loose, lookups, norm, squash};

/// The LaunchBox platform each console's games are filed under.
pub fn launchbox_platform(p: Platform) -> &'static str {
    match p {
        Platform::Gba => "Nintendo Game Boy Advance",
        Platform::Gb => "Nintendo Game Boy",
        Platform::Gbc => "Nintendo Game Boy Color",
        Platform::Gg => "Sega Game Gear",
        Platform::Nes => "Nintendo Entertainment System",
        Platform::Sms => "Sega Master System",
        Platform::Md => "Sega Genesis",
        Platform::Snes => "Super Nintendo Entertainment System",
    }
}

fn platform_of(name: &str) -> Option<Platform> {
    Platform::ALL.into_iter().find(|p| launchbox_platform(*p) == name)
}

/// The consoles a game is also looked for under: a Game Boy Color game is
/// often filed under Game Boy, and the other way round.
fn also(p: Platform) -> &'static [Platform] {
    match p {
        Platform::Gb => &[Platform::Gbc],
        Platform::Gbc => &[Platform::Gb],
        _ => &[],
    }
}

/// The regions a picture is preferred from, in order; a picture with no
/// region comes before a Japanese one.
const REGIONS: [Option<&str>; 8] = [
    Some("Europe"),
    Some("United Kingdom"),
    Some("PAL"),
    Some("World"),
    Some("North America"),
    Some("United States"),
    None,
    Some("Japan"),
];

const OVERVIEW_MAX: usize = 700;

/// A game on the shelves: its console and its ROM's name without the
/// extension.
pub type Game = (Platform, String);

/// What LaunchBox has for a game.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// The game's details file, `key=value` lines.
    pub details: String,
    pub front: Option<String>,
    pub back: Option<String>,
    pub cart: Option<String>,
    pub title: Option<String>,
    pub play: Option<String>,
}

#[derive(Default)]
struct Fields {
    fields: Vec<(String, String)>,
}

impl Fields {
    fn get(&self, key: &str) -> &str {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map_or("", |(_, v)| v.as_str())
    }
}

/// The details of a game, as `tools/fetch_art.py` writes them.
fn details_text(g: &Fields) -> String {
    let one = |k: &str| one_line(g.get(k));
    let year = {
        let date: String = g.get("ReleaseDate").chars().take(4).collect();
        if date.is_empty() {
            one("ReleaseYear")
        } else {
            date
        }
    };
    let rating = g
        .get("CommunityRating")
        .parse::<f64>()
        .map(|r| format!("{r:.2}"))
        .unwrap_or_default();
    let rows = [
        ("name", one("Name")),
        ("year", year),
        ("genre", one("Genres").replace(';', ",")),
        ("developer", one("Developer").replace(';', ",")),
        ("publisher", one("Publisher").replace(';', ",")),
        ("players", one("MaxPlayers")),
        (
            "coop",
            if g.get("Cooperative").eq_ignore_ascii_case("true") {
                "yes".into()
            } else {
                String::new()
            },
        ),
        ("rating", rating),
        ("votes", one("CommunityRatingCount")),
        (
            "esrb",
            one("ESRB").split(" - ").next().unwrap_or("").to_string(),
        ),
        ("overview", clip(g.get("Overview"), OVERVIEW_MAX)),
    ];
    rows.iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}={v}\n"))
        .collect()
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clip(s: &str, n: usize) -> String {
    let s = one_line(s);
    if s.chars().count() <= n {
        return s;
    }
    let head: String = s.chars().take(n).collect();
    let cut = head.rsplit_once(". ").map_or(head.as_str(), |(a, _)| a);
    let kept = if cut.chars().count() > n / 2 {
        cut
    } else {
        head.rsplit_once(' ').map_or(head.as_str(), |(a, _)| a)
    };
    format!("{kept}\u{2026}")
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Kind {
    Front,
    Back,
    Cart,
    Title,
    Play,
}

fn kind_of(image_type: &str) -> Option<(Kind, bool)> {
    match image_type {
        "Box - Front" => Some((Kind::Front, false)),
        "Box - Back" => Some((Kind::Back, false)),
        "Cart - Front" => Some((Kind::Cart, false)),
        "Fanart - Cart - Front" => Some((Kind::Cart, true)),
        "Screenshot - Game Title" => Some((Kind::Title, false)),
        "Screenshot - Gameplay" => Some((Kind::Play, false)),
        _ => None,
    }
}

fn region_rank(region: &str) -> usize {
    let region = (!region.is_empty()).then_some(region);
    REGIONS
        .iter()
        .position(|r| *r == region)
        .unwrap_or(REGIONS.len())
}

struct Reading<'a> {
    wanted: &'a [Game],
    keys: HashSet<String>,
    names: HashMap<Platform, HashMap<String, Vec<String>>>,
    games: HashMap<String, (Platform, u32, Fields)>,
    /// Once the pictures start: each game's match, and its best picture of
    /// each kind so far, as (fan art, region rank, file name).
    matched: Option<HashMap<String, Vec<usize>>>,
    found: HashMap<usize, String>,
    best: HashMap<(String, Kind), (bool, usize, String)>,
}

impl<'a> Reading<'a> {
    fn new(wanted: &'a [Game]) -> Self {
        let keys = wanted.iter().flat_map(|(_, stem)| lookups(stem)).collect();
        Reading {
            wanted,
            keys,
            names: HashMap::new(),
            games: HashMap::new(),
            matched: None,
            found: HashMap::new(),
            best: HashMap::new(),
        }
    }

    fn name(&mut self, platform: Platform, key: String, dbid: &str) {
        if self.keys.contains(&key) {
            self.names
                .entry(platform)
                .or_default()
                .entry(key)
                .or_default()
                .push(dbid.to_string());
        }
    }

    fn game(&mut self, g: Fields) {
        let Some(platform) = platform_of(g.get("Platform")) else {
            return;
        };
        let dbid = g.get("DatabaseID").to_string();
        let name = g.get("Name").to_string();
        for key in [norm(&name), loose(&name), squash(&name)] {
            self.name(platform, key, &dbid);
        }
        let votes = g.get("CommunityRatingCount").parse().unwrap_or(0);
        let mut g = g;
        // Only what the details file keeps.
        for (k, v) in &mut g.fields {
            if k == "Overview" {
                *v = clip(v, OVERVIEW_MAX);
            }
        }
        self.games.insert(dbid, (platform, votes, g));
    }

    fn other_name(&mut self, g: &Fields) {
        let dbid = g.get("DatabaseID");
        let Some(&(platform, _, _)) = self.games.get(dbid) else {
            return;
        };
        let alt = g.get("AlternateName");
        for key in [norm(alt), squash(alt)] {
            self.name(platform, key, dbid);
        }
    }

    /// The game a ROM is: the entry with the most votes among those its
    /// name finds, which is almost always the main release.
    fn match_one(&self, platform: Platform, stem: &str) -> Option<String> {
        let keys = lookups(stem);
        for p in std::iter::once(platform).chain(also(platform).iter().copied()) {
            let Some(names) = self.names.get(&p) else {
                continue;
            };
            let (first, dash) = keys.split_at(3.min(keys.len()));
            let ids = first
                .iter()
                .find_map(|k| names.get(k).filter(|v| !v.is_empty()))
                .or_else(|| dash.first().and_then(|k| names.get(k)));
            if let Some(ids) = ids {
                // The first with the most votes, as Python's max() picks.
                let mut best: Option<(&String, u32)> = None;
                for id in ids {
                    let votes = self.games.get(id).map_or(0, |g| g.1);
                    if best.is_none_or(|(_, v)| votes > v) {
                        best = Some((id, votes));
                    }
                }
                return best.map(|(id, _)| id.clone());
            }
        }
        None
    }

    /// All the games' names are in: match the wanted games and let go of
    /// every other game.
    fn match_all(&mut self) {
        let mut matched: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, (platform, stem)) in self.wanted.iter().enumerate() {
            if let Some(dbid) = self.match_one(*platform, stem) {
                matched.entry(dbid.clone()).or_default().push(i);
                self.found.insert(i, dbid);
            }
        }
        self.names.clear();
        self.games.retain(|dbid, _| matched.contains_key(dbid));
        self.matched = Some(matched);
    }

    fn image(&mut self, g: &Fields) {
        if self.matched.is_none() {
            self.match_all();
        }
        let dbid = g.get("DatabaseID");
        if !self.matched.as_ref().is_some_and(|m| m.contains_key(dbid)) {
            return;
        }
        let Some((kind, fan)) = kind_of(g.get("Type")) else {
            return;
        };
        let file = g.get("FileName");
        if file.is_empty() {
            return;
        }
        let rank = (fan, region_rank(g.get("Region")));
        let key = (dbid.to_string(), kind);
        if self
            .best
            .get(&key)
            .is_none_or(|(f, r, _)| rank < (*f, *r))
        {
            self.best.insert(key, (rank.0, rank.1, file.to_string()));
        }
    }

    fn finish(mut self) -> HashMap<Game, Found> {
        if self.matched.is_none() {
            self.match_all();
        }
        let mut out = HashMap::new();
        for (i, dbid) in &self.found {
            let Some((_, _, fields)) = self.games.get(dbid) else {
                continue;
            };
            let pick = |k: Kind| self.best.get(&(dbid.clone(), k)).map(|b| b.2.clone());
            out.insert(
                self.wanted[*i].clone(),
                Found {
                    details: details_text(fields),
                    front: pick(Kind::Front),
                    back: pick(Kind::Back),
                    cart: pick(Kind::Cart),
                    title: pick(Kind::Title),
                    play: pick(Kind::Play),
                },
            );
        }
        out
    }
}

/// What LaunchBox has for each of `wanted` that it lists, read from its
/// Metadata.xml in `xml`. Stops with an error once `cancel` is set.
pub fn read(
    xml: impl BufRead,
    wanted: &[Game],
    cancel: &AtomicBool,
) -> io::Result<HashMap<Game, Found>> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut state = Reading::new(wanted);
    // The element being read (Game, GameAlternateName or GameImage), the
    // field inside it, and the fields so far.
    let mut top: Option<Vec<u8>> = None;
    let mut field: Option<String> = None;
    let mut value = String::new();
    let mut fields = Fields::default();
    let mut depth = 0u32;
    let mut events = 0u32;
    loop {
        events = events.wrapping_add(1);
        if events.is_multiple_of(4096) && cancel.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "stopped"));
        }
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        match event {
            Event::Start(e) => {
                depth += 1;
                match depth {
                    2 => {
                        let name = e.name().as_ref().as_bytes().to_vec();
                        if matches!(
                            name.as_slice(),
                            b"Game" | b"GameAlternateName" | b"GameImage"
                        ) {
                            top = Some(name);
                            fields = Fields::default();
                        }
                    }
                    3 if top.is_some() => {
                        field = Some(e.name().as_ref().to_string());
                        value.clear();
                    }
                    _ => {}
                }
            }
            Event::Text(t) if field.is_some() => value.push_str(&t),
            Event::CData(t) if field.is_some() => value.push_str(&t.xml10_content()),
            Event::GeneralRef(r) if field.is_some() => match r.resolve_char_ref() {
                Ok(Some(c)) => value.push(c),
                _ => value.push_str(quick_xml::escape::resolve_xml_entity(&r).unwrap_or("")),
            },
            Event::End(_) => {
                match depth {
                    3 => {
                        if let Some(k) = field.take() {
                            fields.fields.push((k, std::mem::take(&mut value)));
                        }
                    }
                    2 => match top.take().as_deref() {
                        Some(b"Game") => state.game(std::mem::take(&mut fields)),
                        Some(b"GameAlternateName") => state.other_name(&fields),
                        Some(b"GameImage") => state.image(&fields),
                        _ => {}
                    },
                    _ => {}
                }
                depth = depth.saturating_sub(1);
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(state.finish())
}
