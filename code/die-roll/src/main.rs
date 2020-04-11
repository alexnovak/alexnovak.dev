// In main.rs
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Error, Read};

// Slapdash code to grab a random u32 value.
fn get_random_int() -> Result<u32, Error> {
    let urandom = File::open("/dev/urandom")?;
    // Take gives us a handle that, when read, gives us only n bytes. 1 in this case.
    let mut handle = urandom.take(1);
    let mut buf = [0_u8; 4];
    handle.read(&mut buf)?;
    // Snip off all but the last three bits.
    // I won't lie that it took me a few guesses for which bits to snip.
    // Endianness is hard.
    buf[0] &= 0b00000111;
    // Rust hackery to turn four u8s into a u32.
    let res = u32::from_le_bytes(buf);
    Ok(res)
}

// This is just our die roll as a function.
fn die_roll(random_int: u32) -> u32 {
    random_int % 6 + 1
}

// Run some specified number of trials of our die roll experiment,
// keeping the frequency of our results in a map.
fn get_longrunning_frequency(trials: u32) -> Result<BTreeMap<u32, u32>, Error> {
    let mut frequency: BTreeMap<u32, u32> = BTreeMap::new();
    for _ in 0..trials {
        let number = get_random_int()?;
        let roll = die_roll(number);
        // Little weird rust hack, if the entry for a value doesn't exist, insert 0.
        let counter = frequency.entry(roll).or_insert(0);
        *counter += 1;
    }
    Ok(frequency)
}

fn main() -> Result<(), Error> {
    let trials = 10_000;
    let frequency = get_longrunning_frequency(trials)?;
    for (value, appearances) in frequency {
        println!(
            "Value: {}, frequency: {}, percentage: {}%",
            value,
            appearances,
            100.0 * appearances as f64 / trials as f64
        );
    }
    Ok(())
}
