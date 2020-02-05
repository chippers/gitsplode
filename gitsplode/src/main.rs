use anyhow::Error;
use git::init;

fn main() -> Result<(), Error> {
    let version = git::version();
    println!("git version: {}", version);

    let git = init().unwrap();
    dbg!(git);

    Ok(())
}
