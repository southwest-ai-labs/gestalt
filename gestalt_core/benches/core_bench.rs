use criterion::{criterion_group, criterion_main, Criterion};
use gestalt_core::db::sqlite::SqliteDb;
use gestalt_core::search::bm25::{tokenize, Bm25Index};
use std::hint::black_box;

fn bench_tokenization(c: &mut Criterion) {
    let sample_text = "The quick brown fox jumps over the lazy dog. \
        pub fn process_event(event: &Event) -> Result<Response, Error> { \
            match event.kind() { \
                EventKind::TaskDispatched => handle_dispatch(event), \
                EventKind::StateUpdated => update_state_db(event), \
                _ => Ok(Response::Ignored), \
            } \
        }";

    c.bench_function("tokenization_sample_code", |b| {
        b.iter(|| {
            let tokens = tokenize(black_box(sample_text));
            black_box(tokens)
        });
    });
}

fn bench_bm25(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25");

    let docs = vec![
        (
            "doc1",
            "src/agent.rs",
            "pub struct GestaltAgent that coordinates multi-agent workflows and task execution",
        ),
        (
            "doc2",
            "src/state.rs",
            "SQLite StateDb manages transactional event logs, locks, and worktree state",
        ),
        (
            "doc3",
            "src/search.rs",
            "BM25 lexical index with reciprocal rank fusion for hybrid vector and keyword search",
        ),
        (
            "doc4",
            "src/bus.rs",
            "Event bus for agent to agent communication and reactive wakeups",
        ),
        (
            "doc5",
            "src/acp.rs",
            "Agent Client Protocol adapter connecting IDE editors to local Gestalt daemon",
        ),
    ];

    group.bench_function("index_5_docs", |b| {
        b.iter(|| {
            let mut index = Bm25Index::new();
            for (id, path, content) in &docs {
                index.index_document(black_box(id), black_box(path), black_box(content), "code");
            }
            black_box(index)
        });
    });

    let mut pre_indexed = Bm25Index::new();
    for (id, path, content) in &docs {
        pre_indexed.index_document(id, path, content, "code");
    }

    let query_tokens = tokenize("gestalt agent state search");
    group.bench_function("search_ranked", |b| {
        b.iter(|| {
            let results = pre_indexed.search(black_box(&query_tokens), None, 5);
            black_box(results)
        });
    });

    group.finish();
}

fn bench_sqlite_db(c: &mut Criterion) {
    c.bench_function("sqlite_in_memory_open", |b| {
        b.iter(|| {
            let db =
                SqliteDb::open(black_box(":memory:")).expect("failed to open in-memory sqlite db");
            black_box(db)
        });
    });
}

criterion_group!(benches, bench_tokenization, bench_bm25, bench_sqlite_db);
criterion_main!(benches);
