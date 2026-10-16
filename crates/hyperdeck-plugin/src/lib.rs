//! Stream Deck plugin that controls named HyperDeck instances.

pub mod actions;
pub mod contracts;
pub mod kind;
pub mod ops;
pub mod render;
pub mod state;

/// Keep action registration and TypeScript exports linked into the binary.
pub fn force_link() {
    actions::force_link();
    contracts::force_link();
}
