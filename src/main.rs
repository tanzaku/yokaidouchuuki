use dict::dict_search;
use domain::{EXPECTED_MEMORY_11, EXPECTED_MEMORY_14, EXPECTED_MEMORY_14_2, EXPECTED_MEMORY_8};

mod bruteforce;
mod cpu;
mod dict;
mod domain;

fn main() {
    // dict_search(&EXPECTED_MEMORY_8);
    // dict_search(&EXPECTED_MEMORY_11);
    dict_search(&EXPECTED_MEMORY_14);

    // find: [22, 5, 5, 11, 39, 5, 11, 11, 18, 11, 41, 10, 11, 5], 4667.677D7c276
    // find: [11, 22, 5, 5, 33, 11, 17, 23, 18, 5, 5, 35, 4, 11], 7466-789D66!17
    // find: [17, 18, 40, 40, 18, 40, 40, 39, 18, 16, 35, 34, 4, 34], 8DmmDmm.D3!n1n
    // dict_search(&EXPECTED_MEMORY_14_2);
}
