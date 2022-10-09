use std::fmt::{Display, Formatter, Result};

pub struct DisplayFn<F: Fn(&mut Formatter) -> Result>(pub F);

impl<F: Fn(&mut Formatter) -> Result> Display for DisplayFn<F> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.0(f)
    }
}
