use dict::dict_search;
use domain::EXPECTED_MEMORY_14;

mod bruteforce;
mod cpu;
mod dict;
mod domain;

fn main() {
    // dict_search(&EXPECTED_MEMORY_8);
    dict_search(&EXPECTED_MEMORY_14);
}
