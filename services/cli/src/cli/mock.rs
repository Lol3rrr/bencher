use clap::Parser;

#[derive(Parser, Debug)]
pub struct CliMock {
    /// Number of mock benchmarks to generate
    #[clap(long)]
    pub count: Option<usize>,
}