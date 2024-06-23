use signal_hook::consts::signal;
use signal_hook::iterator::Signals;
use std::io::{BufRead, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

fn reader(tx: Sender<String>, enabled: Arc<AtomicBool>) {
    let stdin = std::io::stdin().lock();

    // TODO: Consider non \n delimiters. \0?
    for line in stdin.lines() {
        match line {
            Ok(line) => {
                if enabled.load(Ordering::Relaxed) {
                    if let Err(e) = tx.send(line) {
                        eprintln!("Writer not available, quitting: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error on stdin, quitting: {}", e);
                break;
            }
        }
    }
}

fn signal_handler(tx: Sender<String>, mut signals: Signals, enabled: Arc<AtomicBool>) {
    for signal in signals.forever() {
        let send_string = match signal {
            // TODO: Add other signals + command-line strings
            signal::SIGUSR1 => {
                enabled.store(true, Ordering::Relaxed);
                String::from("===USR1===")
            }
            signal::SIGUSR2 => String::from("===USR2==="),
            _ => panic!("Unhandled signal"),
        };

        tx.send(send_string).expect("Writer not available, panic");
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
    let signals =
        Signals::new([signal::SIGUSR1, signal::SIGUSR2]).expect("Unable to register signals");
    let signals_handle = signals.handle();
    let tx_signals = tx.clone();
    // TODO: Add ability to opt-out of this mode
    let enabled = Arc::new(AtomicBool::new(false));
    let enabled_signals = Arc::clone(&enabled);

    let read = thread::spawn(move || {
        reader(tx, enabled);
    });

    let sig = thread::spawn(move || {
        signal_handler(tx_signals, signals, enabled_signals);
    });

    let write = thread::spawn(move || {
        writer(rx);
    });

    read.join().expect("Error closing reader");
    signals_handle.close();
    sig.join().expect("Error closing signal handler");
    write.join().expect("Error closing writer");
}
