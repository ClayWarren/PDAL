use super::*;
use std::ffi::CString;

impl App {
    pub(super) fn run_rust_kernel(&self, name: &str) -> i32 {
        let name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                eprintln!("Error: kernel name contains an interior NUL byte");
                return 1;
            }
        };
        let args = match self
            .command_args
            .iter()
            .map(|arg| CString::new(arg.as_str()))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(args) => args,
            Err(_) => {
                eprintln!("Error: command argument contains an interior NUL byte");
                return 1;
            }
        };
        let argv: Vec<_> = args.iter().map(|arg| arg.as_ptr()).collect();
        unsafe { pdal_capi::pdal_rust_kernel_run(name.as_ptr(), argv.len() as i32, argv.as_ptr()) }
    }

    pub(super) fn run_tile(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!(
                "  pdal tile <input> <output-template> \
                 [--length=N] [--origin_x=X] [--origin_y=Y] [--buffer=N]"
            );
            println!("  the output template must contain a single '#' placeholder");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("tile")
    }

    pub(super) fn run_hausdorff(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal hausdorff <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        self.run_rust_kernel("hausdorff")
    }

    pub(super) fn run_chamfer(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal chamfer <source> <candidate>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        self.run_rust_kernel("chamfer")
    }

    pub(super) fn run_delta(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal delta <source> <candidate> [--detail] [--alldims]");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }
        self.run_rust_kernel("delta")
    }

    pub(super) fn run_tindex(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!("  pdal tindex create --tindex <output> <files...> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --filelist <path> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --glob <pattern> [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --stdin [-f <driver>]");
            println!("  pdal tindex create --tindex <output> --path_prefix <prefix> <files...>");
            println!("  pdal tindex create --tindex <output> --lyr_name <name> <files...>");
            println!("  pdal tindex create <output> <files...> [-f <driver>]");
            println!("  pdal tindex merge --tindex <index> --filespec <output>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("tindex")
    }

    pub(super) fn run_eval(&self) -> i32 {
        if self.help || self.command_args.is_empty() || self.command_help_requested() {
            println!("Usage:");
            println!(
                "  pdal eval <predicted> <truth> --labels=<l1,l2,...> \
                 [--prediction_dim=Classification] [--truth_dim=Classification]"
            );
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        self.run_rust_kernel("eval")
    }
}
