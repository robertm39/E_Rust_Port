use std::env;
use std::hint::black_box;
use std::time::Instant;

const REQUIRED_LENGTHS: [usize; 8] = [256, 1_024, 4_096, 512, 2_048, 384, 1_536, 768];
const TOUCHED_SLOTS: usize = 12;

fn visit(values: &mut [i64], active_len: usize, seed: usize) -> u64 {
    let mut indexes = [0; TOUCHED_SLOTS];
    for (lane, index) in indexes.iter_mut().enumerate() {
        *index = seed
            .wrapping_mul(131)
            .wrapping_add(lane.wrapping_mul(17))
            % active_len;
        values[*index] = values[*index].wrapping_add(i64::try_from(lane + 1).unwrap());
    }

    black_box(values.as_mut_ptr());
    let mut checksum = 0_u64;
    for index in indexes {
        checksum = checksum.wrapping_add(values[index] as u64);
        values[index] = 0;
    }
    checksum
}

fn run_fresh(iterations: usize) -> (u64, usize) {
    let mut checksum = 0_u64;
    for iteration in 0..iterations {
        let required_len = REQUIRED_LENGTHS[iteration % REQUIRED_LENGTHS.len()];
        let mut values = vec![0_i64; required_len];
        checksum = checksum.wrapping_add(visit(&mut values, required_len, iteration));
    }
    (black_box(checksum), iterations)
}

fn run_reused(iterations: usize) -> (u64, usize) {
    let mut checksum = 0_u64;
    let mut growths = 0_usize;
    let mut values = Vec::new();
    for iteration in 0..iterations {
        let required_len = REQUIRED_LENGTHS[iteration % REQUIRED_LENGTHS.len()];
        if values.len() < required_len {
            values.resize(required_len.next_power_of_two(), 0_i64);
            growths += 1;
        }
        checksum = checksum.wrapping_add(visit(&mut values, required_len, iteration));
    }
    (black_box(checksum), growths)
}

fn measure(mode: &str, iterations: usize) -> (u128, u64, usize) {
    let start = Instant::now();
    let (checksum, growths) = match mode {
        "fresh" => run_fresh(iterations),
        "reused" => run_reused(iterations),
        _ => unreachable!(),
    };
    (start.elapsed().as_nanos(), checksum, growths)
}

fn main() {
    let mut args = env::args().skip(1);
    let iterations = args
        .next()
        .expect("missing iteration count")
        .parse::<usize>()
        .expect("invalid iteration count");
    let rounds = args
        .next()
        .expect("missing round count")
        .parse::<usize>()
        .expect("invalid round count");
    assert!(args.next().is_none(), "unexpected benchmark argument");

    println!("mode,round,nanoseconds,checksum,growths");
    for round in 0..rounds {
        let order = if round % 2 == 0 {
            ["fresh", "reused"]
        } else {
            ["reused", "fresh"]
        };
        for mode in order {
            let (nanoseconds, checksum, growths) = measure(mode, iterations);
            println!("{mode},{round},{nanoseconds},{checksum},{growths}");
        }
    }
}
