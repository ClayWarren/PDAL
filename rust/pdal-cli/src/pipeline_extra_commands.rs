use super::*;

impl App {
    pub(super) fn run_random(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal random <output> [--count=N]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("random")
    }

    pub(super) fn run_split(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal split <input> <output> [--length=N | --capacity=N] [--origin_x=X] [--origin_y=Y]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("split")
    }
}
