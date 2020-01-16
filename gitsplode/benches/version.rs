#![feature(test)]

extern crate test;

use test::Bencher;

#[bench]
fn version(b: &mut Bencher) {
    b.iter(|| git::version())
}
