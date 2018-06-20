extern crate byteorder;

#[macro_use]
extern crate structopt;

mod perfdata;

use std::path::PathBuf;
use std::fs::File;
use perfdata::{PerfData, convert};
use std::io::BufReader;

use structopt::StructOpt;


#[derive(StructOpt, Debug)]
#[structopt(name = "hsperf")]
struct Config {
    /// Activate debug mode (display all entries)
    #[structopt(short = "d", long = "debug")]
    debug: bool,

    /// Make size units human readable
    #[structopt(short = "h", long = "human")]
    human: bool,

    /// File to process
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}


fn main() {
    let config = Config::from_args();

    let f = File::open(config.file).expect("Could not open file");
    let data = PerfData::new(&mut BufReader::with_capacity(32768, f)).expect("Could not get perf data");

    let max_mem = data.get_max_mem();
    let used_mem = data.get_used_mem();

    if config.debug {
        for (name, val) in data.entries() {
            println!("[{}]: {}", name, val);
        }
    }

    println!("uptime: {}", data.get_uptime());
    if config.human {
        println!("used_mem: {}", convert(used_mem as f64));
        println!("max_mem: {}", convert(max_mem as f64));
    } else {
        println!("used_mem: {}", used_mem);
        println!("max_mem: {}", max_mem);
    }
    println!("full_gc_count: {}", data.get_gc_full_count());
    println!("full_gc_time: {}", data.get_full_gc());
    println!("total_gc_count: {}", data.get_gc_count());
    println!("total_gc_time: {}", data.get_total_gc());
}


