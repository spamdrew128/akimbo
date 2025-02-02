mod util;
mod position;
mod search;

use crate::{position::{Move, Position}, search::{Engine, go}};
use std::{io, process, time::Instant};

const FEN_STRING: &str = include_str!("../../resources/fens.txt");
const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    println!("akimbo, created by Jamie Whiting");

    // initialise engine
    let mut pos = Position::from_fen(STARTPOS);
    let mut eng = Engine {
        timing: Instant::now(), max_time: 0, abort: false,
        tt: Vec::new(), tt_age: 0,
        htable: Box::new([[[Default::default(); 64]; 8]; 2]),
        plied: Box::new([Default::default(); 96]),
        ntable: Box::new([[0; 64]; 64]),
        stack: Vec::with_capacity(96),
        nodes: 0, qnodes: 0, ply: 0, best_move: Move::default(), seldepth: 0,
    };
    eng.resize_tt(16);

    // bench for OpenBench
    if std::env::args().nth(1).as_deref() == Some("bench") {
        let (mut total_nodes, mut total_time) = (0, 0);
        eng.max_time = 30000;
        let bench_fens = FEN_STRING.split('\n').collect::<Vec<&str>>();
        for fen in bench_fens {
            pos = Position::from_fen(fen);
            let timer = Instant::now();
            go(&pos, &mut eng, false, 11, 1_000_000.0);
            total_time += timer.elapsed().as_millis();
            total_nodes += eng.nodes + eng.qnodes;
        }
        println!("Bench: {total_nodes} nodes {} nps", total_nodes * 1000 / (total_time as u64).max(1));
        return;
    }

    // main uci loop
    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();
        // got EOF, exit (for OpenBench).
        if bytes_read == 0 { break }
        let commands = input.split_whitespace().collect::<Vec<_>>();
        match *commands.first().unwrap_or(&"oops") {
            "uci" => {
                println!("id name akimbo {}\nid author Jamie Whiting", env!("CARGO_PKG_VERSION"));
                println!("option name Threads type spin default 1 min 1 max 1");
                println!("option name Hash type spin default 16 min 1 max 1024");
                println!("option name Clear Hash type button");
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "ucinewgame" => {
                pos = Position::from_fen(STARTPOS);
                eng.clear_tt();
                eng.htable = Box::new([[[Default::default(); 64]; 8]; 2]);
            },
            "setoption" => match commands[..] {
                ["setoption", "name", "Hash", "value", x] => eng.resize_tt(x.parse().unwrap()),
                ["setoption", "name", "Clear", "Hash"] => eng.clear_tt(),
                _ => {}
            },
            "go" => {
                let (mut token, mut times, mut mtg, mut alloc, mut incs) = (0, [0, 0], 25, 1000, [0, 0]);
                let tokens = ["go", "movetime", "wtime", "btime", "movestogo", "winc", "binc"];
                for cmd in commands {
                    if let Some(x) = tokens.iter().position(|&y| y == cmd) { token = x }
                    else if let Ok(val) = cmd.parse::<i64>() {
                        match token {
                            1 => {alloc = val; mtg = 1; times = [val, val]},
                            2 | 3 => times[token - 2] = val.max(0),
                            4 => mtg = val,
                            5 | 6 => incs[token - 5] = val.max(0),
                            _ => {},
                        }
                    }
                }
                let side = usize::from(pos.c);
                let (time, inc) = (times[side], incs[side]);
                if time != 0 { alloc = time.min(time / mtg + 3 * inc / 4) }
                eng.max_time = (alloc * 2).clamp(1, 1.max(time - 10)) as u128;
                go(&pos, &mut eng, true, 64, if mtg == 1 {alloc} else {alloc * 6 / 10} as f64);
            },
            "position" => {
                let (mut fen, mut move_list, mut moves) = (String::new(), Vec::new(), false);
                for cmd in commands {
                    match cmd {
                        "position" | "startpos" | "fen" => {}
                        "moves" => moves = true,
                        _ => if moves { move_list.push(cmd) } else { fen.push_str(&format!("{cmd} ")) }
                    }
                }
                pos = Position::from_fen(if fen.is_empty() { STARTPOS } else { &fen });
                eng.stack.clear();
                for m in move_list {
                    eng.stack.push(pos.hash());
                    let possible_moves = pos.movegen::<true>();
                    for mov in &possible_moves.list[..possible_moves.len] {
                        if m == mov.to_uci() { pos.make(*mov); }
                    }
                }
            },
            "perft" => {
                let (depth, now) = (commands[1].parse().unwrap(), Instant::now());
                let count = perft(&pos, depth);
                let time = now.elapsed().as_micros();
                println!("perft {depth} time {} nodes {count} ({:.2} Mnps)", time / 1000, count as f64 / time as f64);
            },
            "quit" => process::exit(0),
            "eval" => println!("eval: {}cp", pos.eval()),
            _ => {},
        }
    }
}

fn perft(pos: &Position, depth: u8) -> u64 {
    let moves = pos.movegen::<true>();
    let mut positions = 0;
    for &m in &moves.list[0..moves.len] {
        let mut tmp = *pos;
        if tmp.make(m) { continue }
        positions += if depth > 1 { perft(&tmp, depth - 1) } else { 1 };
    }
    positions
}
