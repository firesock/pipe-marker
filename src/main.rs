use std::io::{BufRead, BufWriter, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

fn reader(tx: Sender<String>) {
    let stdin = std::io::stdin().lock();

    for line in stdin.lines() {
        match line {
            Ok(line) => {
                if let Err(e) = tx.send(line) {
                    eprintln!("Writer not available, quitting: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error on stdin, quitting: {}", e);
                break;
            }
        }
    }
}

fn writeln<T>(writer: &mut T, msg: String) -> std::io::Result<()>
where
    T: Write,
{
    // TODO: Handle non-UTF8 systems
    writer.write_all(msg.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn writer(rx: Receiver<String>) {
    let stdout = std::io::stdout().lock();
    // TODO: Work out good buffer size for user responsiveness
    let mut writer = BufWriter::new(stdout);

    for line in rx {
        if let Err(e) = writeln(&mut writer, line) {
            eprintln!("Error on stdout, quitting: {}", e);
            break;
        }
    }
    if let Err(e) = writer.flush() {
        eprintln!("Error flushing stdout during quit: {}", e);
    }
}

fn main() {
    let (tx, rx) = channel();

    let read = thread::spawn(move || {
        reader(tx);
    });

    let write = thread::spawn(move || {
        writer(rx);
    });

    read.join().expect("Error with reader");
    write.join().expect("Error with writer");
}
