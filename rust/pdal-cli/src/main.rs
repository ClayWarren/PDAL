mod app;
mod stage_metadata;

fn main() {
    std::process::exit(app::run(std::env::args().skip(1).collect()));
}
