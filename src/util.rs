use std::error::Error;

pub trait StringErr<T> {
    fn string_err(self) -> Result<T, String>;
}

impl<T, E: Error> StringErr<T> for Result<T, E> {
    fn string_err(self) -> Result<T, String> {
        self.map_err(|err| format!("{}", err))
    }
}
