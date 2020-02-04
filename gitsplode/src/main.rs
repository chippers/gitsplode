use anyhow::Error;
use git::init;
use std::time::Duration;

fn main() -> Result<(), Error> {
    let version = git::version();
    println!("git version: {}", version);

    /*let mut repo = Repository::init("./.git", ".")?;

    dbg!(&repo);

    let mut revs = Revisions::init(&mut repo);

    dbg!(revs.inner.pending);
    dbg!(revs.inner.commit_format);
    dbg!(revs.inner.expand_tabs_in_log_default);

    revs.add_head_to_pending();*/

    //dbg!(revs.inner.pending);

    let mut t = Vec::new();
    //let mut t2 = Vec::new();

    for i in 0..10 {
        t.push(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_nanos(10 - i));
            let git = init();
            println!("{}: {:#?}", i, git);
        }));
    }

    /*    for i in 0..10 {
        t2.push(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_nanos(10 - i));
            let git = git::init2();
            println!("t2: {}: {:#?}", i, git);
        }));
    }*/

    for j in t {
        j.join().unwrap();
    }

    /*
        for j in t2 {
            j.join().unwrap();
        }
    */

    //let mut git = init().expect("unable to init git");
    //let mut git2 = init().expect("unable to init git2");

    Ok(())
}
