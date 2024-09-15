use std::fmt::Display;

use termion::color;

const PROMPT: &str = "π ⇾ ";

pub fn get_prompt() -> String {
    format!("{}{}", color::Fg(color::LightMagenta), PROMPT)
}

pub fn prompt_for_password() -> Result<String, std::io::Error> {
    println!("\n🔑 Enter a passphrase (recommended) or leave empty");
    let password = rpassword::prompt_password(get_prompt())?;
    println!("{}", color::Fg(color::Reset));
    Ok(password)
}

pub fn prompt_for_password_unlock(name: &str) -> Result<String, std::io::Error> {
    println!("\n🔑 Enter keystore passphrase for {}", name);
    let password = rpassword::prompt_password(get_prompt())?;
    println!("{}", color::Fg(color::Reset));
    Ok(password)
}

pub fn prompt_for_password_confirmation() -> Result<String, std::io::Error> {
    println!("🔑 Confirm your passphrase");
    let password = rpassword::prompt_password(get_prompt())?;
    println!("{}", color::Fg(color::Reset));
    Ok(password)
}

pub fn print_var<V: Display>(name: &str, value: V) {
    println!(
        "{}{} {}{}{}",
        color::Fg(color::LightMagenta),
        name,
        color::Fg(color::Green),
        value,
        color::Fg(color::Reset)
    );
}
