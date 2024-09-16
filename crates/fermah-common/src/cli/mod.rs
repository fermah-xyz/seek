pub mod ascii;
pub mod prompts;
pub mod spinner;

pub const VERGEN_TRIPLE: &str = env!("VERGEN_RUSTC_HOST_TRIPLE");
pub const VERGEN_COMMIT: &str = env!("VERGEN_RUSTC_COMMIT_HASH");
pub const VERGEN_TS: &str = env!("VERGEN_BUILD_TIMESTAMP");

#[macro_export]
macro_rules! print_info {
    () => {
        println!(
            "{}\t\t\t\t  {} {}{}",
            termion::color::Fg(termion::color::LightGreen),
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            termion::color::Fg(termion::color::Reset),
        );

        println!(
            "{}\t\t{} . {} . {}{}\n\n",
            termion::color::Fg(termion::color::LightMagenta),
            cli::VERGEN_TRIPLE,
            &cli::VERGEN_COMMIT.to_string()[..7],
            cli::VERGEN_TS,
            termion::color::Fg(termion::color::Reset),
        );
    };
}
