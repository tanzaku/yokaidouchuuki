use dict::dict_search;
use domain::EXPECTED_MEMORY_14;

mod bitset;
mod cpu;
mod dict;
mod domain;
mod opt;
mod pruning;

fn main() {
    // dict_search(&EXPECTED_MEMORY_8);
    // dict_search(&EXPECTED_MEMORY_11);
    dict_search(&EXPECTED_MEMORY_14);
}
