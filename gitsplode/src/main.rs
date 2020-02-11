use anyhow::Error;
use git::{init, Rev};

fn main() -> Result<(), Error> {
    let version = git::version();
    println!("git version: {}", version);

    let git = init().unwrap();
    dbg!(&git);

    let mut revs = Rev::new(&git);
    revs.add_head_to_pending();
    dbg!(&revs.rev_info.total);

    Ok(())
}
