//! What is known about a game, from `Details/<console>/<rom name>.txt`: one
//! `key=value` line per fact, written on a laptop from LaunchBox. Every
//! key is optional and unknown ones are ignored.

use std::path::{Path, PathBuf};

use crate::scan::Cart;

pub const DETAILS_DIR: &str = "Details";

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Details {
    pub name: Option<String>,
    pub year: Option<u16>,
    pub genres: Vec<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    /// The most players at once.
    pub players: Option<u8>,
    pub coop: bool,
    /// Out of 5.
    pub rating: Option<f32>,
    pub votes: Option<u32>,
    /// The age rating; "Not Rated" is no rating.
    pub esrb: Option<String>,
    pub overview: Option<String>,
}

impl Details {
    /// Whether nothing at all was known.
    pub fn is_empty(&self) -> bool {
        *self == Details::default()
    }

    /// Read from the text of a details file.
    pub fn parse(text: &str) -> Details {
        let mut d = Details::default();
        for line in text.trim_start_matches('\u{feff}').lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match key.trim().to_ascii_lowercase().as_str() {
                "name" => d.name = Some(value.to_string()),
                "year" => d.year = value.parse().ok().filter(|y| *y > 0),
                "genre" => {
                    d.genres = value
                        .split(',')
                        .map(str::trim)
                        .filter(|g| !g.is_empty())
                        .map(str::to_string)
                        .collect()
                }
                "developer" => d.developer = Some(value.to_string()),
                "publisher" => d.publisher = Some(value.to_string()),
                "players" => d.players = value.parse().ok().filter(|p| *p > 0),
                "coop" => {
                    d.coop = ["yes", "true", "1"]
                        .iter()
                        .any(|y| value.eq_ignore_ascii_case(y))
                }
                "rating" => {
                    d.rating = value
                        .parse::<f32>()
                        .ok()
                        .filter(|r| r.is_finite())
                        .map(|r| r.clamp(0.0, 5.0))
                }
                "votes" => d.votes = value.parse().ok(),
                "esrb" => {
                    d.esrb =
                        Some(value.to_string()).filter(|e| !e.eq_ignore_ascii_case("not rated"))
                }
                "overview" => d.overview = Some(value.to_string()),
                _ => {}
            }
        }
        d
    }
}

/// Where the details of `cart` are kept under the data root.
pub fn details_path(root: &Path, cart: &Cart) -> PathBuf {
    root.join(DETAILS_DIR)
        .join(cart.platform.dir_name())
        .join(format!("{}.txt", cart.stem))
}

/// The details of `cart`, if it has a file of them with anything in it.
pub fn details_for(root: &Path, cart: &Cart) -> Option<Details> {
    let text = std::fs::read_to_string(details_path(root, cart)).ok()?;
    Some(Details::parse(&text)).filter(|d| !d.is_empty())
}
