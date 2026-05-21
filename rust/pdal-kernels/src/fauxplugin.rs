use crate::{Kernel, KernelArgs, KernelError, KernelSpec};

#[derive(Default)]
pub struct FauxPluginKernel {
    fake_arg: Option<String>,
}

impl FauxPluginKernel {
    pub fn fake_arg(&self) -> Option<&str> {
        self.fake_arg.as_deref()
    }
}

impl Kernel for FauxPluginKernel {
    fn spec(&self) -> KernelSpec {
        KernelSpec {
            name: "kernels.fauxplugin",
            description: "Faux Plugin Kernel",
        }
    }

    fn run(&mut self, args: &KernelArgs) -> Result<i32, KernelError> {
        self.fake_arg = args.as_slice().first().cloned();
        println!("FauxPluginKernel running.");
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_optional_positional_fakearg() {
        let mut kernel = FauxPluginKernel::default();
        assert_eq!(kernel.run(&KernelArgs::new(["42"])).unwrap(), 0);
        assert_eq!(kernel.fake_arg(), Some("42"));
    }
}
