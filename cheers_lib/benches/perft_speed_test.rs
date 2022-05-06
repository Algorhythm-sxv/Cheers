use cheers_lib::*;
use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

pub fn criterion_benchmark(c: &mut Criterion) {
    lookup_tables::LookupTables::generate_all(true);
    zobrist::initialise_zobrist_numbers();

    let tt = transposition_table::TranspositionTable::new(transposition_table::TT_DEFAULT_SIZE);
    let mut game = chessgame::ChessGame::new(tt);

    c.bench_function("Perft speed test", |b| {
        b.iter(|| {
            game.perft(5);
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
);
criterion_main!(benches);
