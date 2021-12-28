use dict::dict_search;
use domain::{EXPECTED_MEMORY_11, EXPECTED_MEMORY_14};
use skk::skk_dict_search;

mod bitset;
mod cpu;
mod dict;
mod domain;
mod opt;
mod pruning;
mod skk;

fn main() {
    // dict_search(&EXPECTED_MEMORY_8);
    // dict_search(&EXPECTED_MEMORY_11);
    dict_search(&EXPECTED_MEMORY_14);
    // skk_dict_search();
}
