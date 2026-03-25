use clap::{Args, Command};
use clap_complete::aot::{Shell, generate};
use color_eyre::Result;
use std::io;

#[derive(Args)]
pub struct AutocompleteArgs {
    shell: Shell,
}

pub fn autocomplete(cmd: &mut Command, args: &AutocompleteArgs) -> Result<()> {
    generate(args.shell, cmd, "snlo", &mut io::stdout());
    Ok(())
}
