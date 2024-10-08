use cheers_lib::{
    board::{tb_adapter::MovegenAdapter, Board},
    hash_tables::TranspositionTable,
    options::SearchOptions,
    pyrrhic_rs::TableBases,
    search::{Search, ABORT_SEARCH, NODE_COUNT},
    types::Color,
};

use std::{
    error::Error,
    io::{prelude::*, stdin},
    sync::{atomic::Ordering, Arc, RwLock},
    thread::{self, JoinHandle},
    time::Instant,
};

mod uci;

fn main() -> Result<(), Box<dyn Error>> {
    let mut position = Board::new();
    let mut options = SearchOptions::default();
    let mut chess_960 = false;

    let mut tbs = None;
    let mut running_thread: Option<JoinHandle<_>> = None;

    let mut tt = Arc::new(RwLock::new(TranspositionTable::new(options.tt_size_mb)));
    let mut pre_history = Vec::new();

    if std::env::args().nth(1) == Some(String::from("bench")) {
        let depth = std::env::args()
            .nth(2)
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(15);
        let bench_game = position;
        let search = Search::new(bench_game)
            .max_depth(Some(depth))
            .tt_size_mb(options.tt_size_mb)
            .output(false);
        let start = Instant::now();
        search.smp_search();
        let end = Instant::now();
        let time = end - start;

        let nodes = NODE_COUNT.load(Ordering::Relaxed);
        let nps = (nodes as f64 / time.as_secs_f64()) as usize;
        println!("{nodes} nodes {nps} nps");
        return Ok(());
    }

    for line in stdin().lock().lines() {
        let cmd = match uci::parse_uci_command(line?) {
            Ok(cmd) => cmd,
            Err(uci::UciParseError::Empty) => {
                continue;
            }
            Err(uci::UciParseError::Other(e)) => {
                eprintln!("{e}");
                continue;
            }
        };

        match cmd {
            uci::UciCommand::Uci => {
                println!("id name Cheers");
                println!("id author Algorhythm");
                uci::print_uci_options();
                println!("uciok");
            }
            uci::UciCommand::IsReady => println!("readyok"),
            uci::UciCommand::SetOption(opt) => match opt {
                uci::UciOption::Hash(mb) => {
                    options.tt_size_mb = mb;
                    tt.write().unwrap().set_size(mb);
                }
                uci::UciOption::Threads(n) => options.threads = n,
                uci::UciOption::UCI_Chess960(x) => chess_960 = x,
                uci::UciOption::SyzygyPath(p) => {
                    // drop old TBs
                    tbs = None;
                    if &p != "<empty>" {
                        match TableBases::<MovegenAdapter>::new(p.clone()) {
                            Ok(t) => {
                                println!("info string loaded syzygy tablebases from {p}");
                                tbs = Some(t)
                            }
                            Err(e) => eprintln!("Failed to load tablebases: {e:?}"),
                        }
                    }
                }
                uci::UciOption::NmpDepth(n) => options.nmp_depth = n,
                uci::UciOption::NmpConstReduction(n) => options.nmp_const_reduction = n,
                uci::UciOption::NmpLinearDivisor(n) => options.nmp_linear_divisor = n,
                uci::UciOption::SeePruningDepth(n) => options.see_pruning_depth = n,
                uci::UciOption::SeeCaptureMargin(n) => options.see_capture_margin = n,
                uci::UciOption::SeeQuietMargin(n) => options.see_quiet_margin = n,
                uci::UciOption::PvsFulldepth(n) => options.pvs_fulldepth = n,
                uci::UciOption::DeltaPruningMargin(n) => options.delta_pruning_margin = n,
                uci::UciOption::FpMargin1(n) => options.fp_margin_1 = n,
                uci::UciOption::FpMargin2(n) => options.fp_margin_2 = n,
                uci::UciOption::FpMargin3(n) => options.fp_margin_3 = n,
                uci::UciOption::RfpDepth(n) => options.rfp_depth = n,
                uci::UciOption::RfpMargin(n) => options.rfp_margin = n,
                uci::UciOption::RfpImprovingMargin(n) => options.rfp_improving_margin = n,
                uci::UciOption::LmpDepth(n) => options.lmp_depth = n,
                uci::UciOption::HistoryLmrDivisor(n) => options.history_lmr_divisor = n,
                uci::UciOption::IirDepth(n) => options.iir_depth = n,
            },
            uci::UciCommand::UciNewGame => {
                position = Board::new();
                pre_history.clear();
                tt = Arc::new(RwLock::new(TranspositionTable::new(options.tt_size_mb)));
            }
            uci::UciCommand::Position { fen, moves } => {
                match fen {
                    Some(fen) => position = Board::from_fen(fen).unwrap(),
                    None => position = Board::new(),
                }
                pre_history.clear();
                for m in moves {
                    pre_history.push(position.hash());
                    position.make_move(m);
                }
            }
            uci::UciCommand::Go {
                wtime,
                btime,
                winc,
                binc,
                movestogo,
                depth,
                nodes,
                movetime,
                infinite,
                perft,
            } => {
                // make sure the previous search terminates before starting a new one
                if let Some(handle) = running_thread.take() {
                    // retrieve the TB handle to re-use
                    if let Some(tb) = handle.join().expect("Search thread crashed") {
                        tbs = Some(tb);
                    }
                }
                if let Some(depth) = perft {
                    position.perft(depth);
                    continue;
                }
                let movetime = if infinite {
                    None
                } else {
                    match movetime {
                        Some(time) => Some((time, time)),
                        None => match movestogo {
                            Some(n) => {
                                if position.current_player() == Color::White {
                                    // add a 50ms margin to avoid timeouts
                                    Some((
                                        (wtime.unwrap().max(0) as usize - 50) / n,
                                        (wtime.unwrap().max(0) as usize - 50) / n,
                                    ))
                                } else {
                                    // add a 50ms margin to avoid timeouts
                                    Some((
                                        (btime.unwrap().max(0) as usize - 50) / n,
                                        (btime.unwrap().max(0) as usize - 50) / n,
                                    ))
                                }
                            }
                            None => {
                                if position.current_player() == Color::White {
                                    move_time(wtime, winc)
                                } else {
                                    move_time(btime, binc)
                                }
                            }
                        },
                    }
                };

                // extract the TBs so the search thread can be the sole owner
                let tb = tbs.take();

                let mut search = Search::new_with_tt(position, tt.clone())
                    .tt_size_mb(options.tt_size_mb)
                    .tablebases(tb)
                    .pre_history(pre_history.clone())
                    .max_nodes(nodes)
                    .max_depth(depth)
                    .options(options)
                    .output(true)
                    .chess_960(chess_960);
                search.max_time_ms = movetime;

                running_thread = Some(thread::spawn(move || engine_thread(search).unwrap()));
            }
            uci::UciCommand::Fen => println!("{}", position.fen()),
            uci::UciCommand::Stop => ABORT_SEARCH.store(true, Ordering::Relaxed),
            uci::UciCommand::Quit => {
                ABORT_SEARCH.store(true, Ordering::Relaxed);
                break;
            }
        }
    }
    Ok(())
}

fn engine_thread(search: Search) -> Result<Option<TableBases<MovegenAdapter>>, Box<dyn Error>> {
    ABORT_SEARCH.store(false, Ordering::Relaxed);
    NODE_COUNT.store(0, Ordering::Relaxed);

    let (_, pv, tbs) = search.smp_search();

    println!("bestmove {}", pv[0].coords());

    Ok(tbs)
}

fn move_time(time_millis: Option<isize>, inc_millis: Option<isize>) -> Option<(usize, usize)> {
    let (time, inc) = match (time_millis, inc_millis) {
        (None, None) => return None,
        (t, i) => (
            t.unwrap_or(0).max(0) as usize,
            i.unwrap_or(0).max(0) as usize,
        ),
    };
    if time < inc {
        Some((time / 20, time / 2))
    } else {
        Some((time / 20 + inc / 2, time / 2))
    }
}
