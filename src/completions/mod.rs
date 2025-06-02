use clap::Command;
use clap_complete::{generate, Generator, Shell};
use std::io;

pub fn generate_completions<G: Generator>(gen: G, app: &mut Command) {
    generate(gen, app, app.get_name().to_string(), &mut io::stdout());
}

pub fn generate_shell_completions(shell: Shell) -> Result<String, Box<dyn std::error::Error>> {
    let mut app = crate::cli::Cli::command();
    let mut output = Vec::new();
    
    match shell {
        Shell::Bash => generate(clap_complete::shells::Bash, &mut app, "archbox", &mut output),
        Shell::Zsh => generate(clap_complete::shells::Zsh, &mut app, "archbox", &mut output),
        Shell::Fish => generate(clap_complete::shells::Fish, &mut app, "archbox", &mut output),
        Shell::PowerShell => generate(clap_complete::shells::PowerShell, &mut app, "archbox", &mut output),
        Shell::Elvish => generate(clap_complete::shells::Elvish, &mut app, "archbox", &mut output),
        _ => return Err("Unsupported shell".into()),
    }
    
    Ok(String::from_utf8(output)?)
}

pub fn install_completions(shell: Shell) -> Result<(), Box<dyn std::error::Error>> {
    let completions = generate_shell_completions(shell)?;
    
    let completion_dir = match shell {
        Shell::Bash => dirs::home_dir()
            .map(|d| d.join(".local/share/bash-completion/completions")),
        Shell::Zsh => dirs::home_dir()
            .map(|d| d.join(".local/share/zsh/site-functions")),
        Shell::Fish => dirs::home_dir()
            .map(|d| d.join(".config/fish/completions")),
        _ => None,
    };
    
    if let Some(dir) = completion_dir {
        std::fs::create_dir_all(&dir)?;
        let file_path = dir.join("archbox");
        std::fs::write(file_path, completions)?;
        println!("Completions installed for {:?}", shell);
    } else {
        println!("Manual installation required for {:?}:", shell);
        println!("{}", completions);
    }
    
    Ok(())
}