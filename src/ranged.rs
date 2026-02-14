use core::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use tower_lsp::lsp_types;

#[derive(PartialEq, Debug, thiserror::Error)]
pub struct Ranged<E: Error> {
    pub error: E,
    pub range: Option<lsp_types::Range>,
}

pub fn ranged<E: Error>(error: E, md_position: Option<&markdown::unist::Position>) -> Ranged<E> {
    Ranged {
        error,
        range: md_position
            .map(|p| lsp_types::Range::new(lsp_position(&p.start), lsp_position(&p.end))),
    }
}

/// Converts between library position
/// `lsp_types::Position` is just a coordinate, where as `markdown::unist::Position` maps to `lsp_types::Reange`
fn lsp_position(point: &markdown::unist::Point) -> lsp_types::Position {
    lsp_types::Position::new(
        (point.line - 1).try_into().expect("Reasonable file size"),
        (point.column - 1).try_into().expect("Reasonable file size"),
    )
}

impl<E: Error> Display for Ranged<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}
