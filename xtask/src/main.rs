use getopt3::hideBin;
use std::env::args;
use xtasks::ops::cmd;

fn main() {
    println!("Hello, world!");

    let opts = getopt3::new(hideBin(args()), "abc").expect("Invalid arguments");

    if let Some(_) = opts.options.get(&'d') {
        println!("Build/Deploy smart contract");

        let _ = cmd!("cargo", "build-sbf").read().unwrap();

        let output = cmd!("solana", "program", "deploy", "--program-id", "./tree_program/keypair.json", "target/deploy/tree_program.so").read().unwrap();
        println!("Result {}", output);

        return;
    }

    // if let Ok(g) = rc {
    //    // command line options parsed sucessfully
    //    if let Some(arg) = g.options.get(&'b') {
    //       // handle b argument stored in arg
    //    };
    // };
}
