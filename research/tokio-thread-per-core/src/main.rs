use affinity::*;
use crossbeam::channel::unbounded;
use std::{thread, time::Instant};
use tokio::runtime;

fn main() {
    println!("Hello, world!");

    let (s, r) = unbounded();

    println!("-> Main Thread on CPU {}", unsafe { libc::sched_getcpu() });

    let handle1 = thread::spawn(move || {
        set_thread_affinity(&[0]).unwrap();
        let rt = runtime::Builder::new_current_thread()
            .build()
            .expect("Failed to build tokio");
        rt.block_on(async {
            println!("-> Thread 1 on CPU {}", unsafe { libc::sched_getcpu() });
            s.send((Instant::now(), "Hello, world!")).unwrap();
        });
    });

    let handle2 = thread::spawn(move || {
        set_thread_affinity(&[2]).unwrap();
        let rt = runtime::Builder::new_current_thread()
            .build()
            .expect("Failed to build tokio");
        rt.block_on(async {
            println!("-> Thread 2 on CPU {}", unsafe { libc::sched_getcpu() });
            loop {
                let msg = r.recv();
                match msg {
                    Ok((time, msg)) => {
                        let elapsed = time.elapsed();
                        println!("{elapsed:?} {msg}");
                    }
                    _ => {}
                }
            }
        });
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
