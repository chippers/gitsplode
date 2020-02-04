use anyhow::Error;
use git::init;
use std::time::Duration;

fn main() -> Result<(), Error> {
    let version = git::version();
    println!("git version: {}", version);

    let mut t = Vec::new();

    for i in 0..10 {
        t.push(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_nanos(10 - i));
            let git = init();
            println!("{}: {:#?}", i, git);
        }));
    }

    for j in t {
        j.join().unwrap();
    }

    //let mut git = init().expect("unable to init git");
    //let mut git2 = init().expect("unable to init git2");

    Ok(())
}
