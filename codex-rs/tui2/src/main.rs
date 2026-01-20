use clap::Parser;
use codex_arg0::arg0_dispatch_or_else;
use codex_common::CliConfigOverrides;
use codex_tui2::Cli;
use codex_tui2::run_main;

#[derive(Parser, Debug)]
struct TopCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    inner: Cli,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        let top_cli = TopCli::parse();
        let mut inner = top_cli.inner;
        inner
            .config_overrides
            .raw_overrides
            .splice(0..0, top_cli.config_overrides.raw_overrides);

        // ADR-002: tui2 is experimental upstream scaffold, not a replacement for tui
        eprintln!(
            "\x1b[33m[tui2] Experimental: chat-only mode. spec-kit commands not supported.\x1b[0m"
        );
        eprintln!(
            "\x1b[33m[tui2] Use 'code' binary for /speckit.auto and golden-path workflows.\x1b[0m\n"
        );

        let exit_info = run_main(inner, codex_linux_sandbox_exe).await?;
        let token_usage = exit_info.token_usage;
        if !token_usage.is_zero() {
            println!("{}", codex_core::protocol::FinalOutput::from(token_usage),);
        }
        Ok(())
    })
}
