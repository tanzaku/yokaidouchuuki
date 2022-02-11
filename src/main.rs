use config::SEARCH_TARGET;
use enumeration::enumeration;

mod backward;
mod bitset;
mod config;
mod cpu;
mod domain;
mod enumeration;
mod forward1;
mod forward2;
mod target;
mod time;

fn main() {
    enumeration(SEARCH_TARGET);
}
