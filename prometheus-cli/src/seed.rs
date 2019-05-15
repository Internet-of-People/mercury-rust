use failure::Fallible;
use log::*;

pub fn generate_phrase() -> Vec<String> {
    let new_bip39_phrase = keyvault::Seed::generate_bip39();
    let words = new_bip39_phrase.split(' ');
    words.map(|s| s.to_owned()).collect()
}

pub fn show_generated_phrase() {
    let words = generate_phrase();
    warn!(
        r#"Make sure you back these words up somewhere safe
and run the 'restore vault' command of this application first!"#
    );
    words.iter().enumerate().for_each(|(i, word)| info!("    {:2}: {}", i + 1, word));
}

pub fn read_phrase() -> Fallible<String> {
    use std::io::BufRead;
    use std::io::Write;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();
    stdout_lock.write_fmt(format_args!(
        "Please type the words you backed up one-by-one pressing enter after each:\n"
    ))?;

    let mut words = Vec::with_capacity(24);
    for i in 1..=24 {
        loop {
            let mut buffer = String::with_capacity(10);
            stdout_lock.write_fmt(format_args!("  {:2}> ", i))?; // no newline at the end for this prompt!
            stdout_lock.flush()?; // without this, nothing is written on the console
            stdin_lock.read_line(&mut buffer)?;
            buffer = buffer.trim().to_owned();
            if keyvault::Seed::check_word(&buffer) {
                words.push(buffer);
                break;
            } else {
                stdout_lock.write_fmt(format_args!(
                    "{} is not in the dictionary, please retry entering it\n",
                    buffer
                ))?;
            }
        }
    }
    let phrase = words.join(" ");

    debug!("You entered: {}", phrase);

    Ok(phrase)
}
