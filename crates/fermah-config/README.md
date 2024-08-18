# Overview

This crate provides a simple way to create profiles for different configurations of your application. It is designed to
be used by any crate that uses JSON configuration files, and it is flexible enough to be used in CLI through the usage
of
a provided Subcommand to embed within your CLI application arguments. Profiles are indexed with a composite key:

```rust
pub struct ProfileKey {
    pub network: Network,
    pub name: String,
}
```

Features include:

- Profile creation based on an existing 'template' profile
- CLI integration through `clap`
- Support for merging configuration arguments with the actual configuration, reducing the number of arguments needed
- Supports network, profile type and profile name for building the paths
- Currently only supports JSON configuration files

The crate provides the following CLI commands:

- `list` - Lists all profiles
- `get` - Gets a profile by key
- `set` - Sets a profile by key and data
- `delete` - Deletes a profile by name

# Usage

To use this crate, add it to your `Cargo.toml`:

```toml
[dependencies]
fermah-config = { workspace = true }
```

The crate is coupled to `clap` crate for CLI applications. It is meant to be used as a subcommand this way:

```rust
// Custom configuration arguments for the actual config
#[derive(Serialize, Deserialize, Parser, Debug)]
pub struct MyConfigArgs {
    /// My configuration option
    #[arg(short = 'n')]
    name: String,
}

// Actual CLI configuration
#[derive(Serialize, Deserialize)]
pub struct MyConfig {
    name: String,
    other_data: String,
}

// Implement mergable for the configuration arguments
impl MergableArgs for MyConfigArgs {
    type MergeType = MyConfig;

    // Merge the configuration arguments with the actual 'complex' configuration
    fn merge(&self, other: Self::MergeType) -> Self::MergeType {
        Self::MergeType {
            name: self.name.clone(),
            ..other
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum MyCLICommands {
    #[command(alias = "s")]
    Start {
        profile: fermah_config::profile_key::ProfileKey, // Contains args parsing for network and profile name
        // ... other arguments
    },
    #[command(alias = "cfg")]
    Config {
        #[command(subcommand)]
        profiles: ProfileCommands<MyConfigArgs>, // Embed the profile commands
    },
}

fn main() {
    // .. parse the CLI arguments

    // Configuration base directory
    let config_dir = PathBuf::from("config");

    match args.commands {
        MyCLICommands::Config { profiles } => {
            StdoutTelemetry::default().init();
            profiles.run(&profiles, ProfileType::Matchmaker, config_dir).await?;
        }
        MyCLICommands::Start { profile } => {
            let config_profile = MyConfig::from_profile(
                &config_dir,
                ProfileType::Matchmaker,
                &profile_key,
            ).await?;
            // the application configuration is at config_profile.config
        }
        _ => {
            // .. handle other commands
        }
    }
}
```

In summary:

- The `MyConfigArgs` struct is a simplified version of the `MyConfig` struct, containing only the arguments that can be
  passed through the CLI.
- The `MyConfig` struct is the actual configuration that will be used by the application.
- The `Merge` trait is used to merge the `MyConfigArgs` struct with the `MyConfig` struct.
- The `MyCLICommands` enum contains the subcommands that will be used by the application, including the `Config`
  subcommand
  that embeds the profile commands, and the new `profile:` argument that is used to pass the profile key to the
  application.
- The `config_dir` variable is the base directory where the configuration files will be stored.
- When matching the config commands, we use the convenient `fermah_config::command::exec` function to execute the
  profile
  commands.