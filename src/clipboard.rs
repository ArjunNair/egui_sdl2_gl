#[derive(Clone, Copy, Debug)]
pub struct Error;
impl core::fmt::Display for Error {
    fn fmt(&self, _f: &mut core::fmt::Formatter) -> core::fmt::Result {
        Ok(())
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub trait ClipboardProvider: Sized {
    fn new() -> Result<Self>;
    fn get_contents(&mut self) -> Result<String>;
    fn set_contents(&mut self, contents: String) -> Result<()>;
    fn clear(&mut self) -> Result<()>;
}

pub struct ClipboardContext {
    contents: String,
}

impl ClipboardProvider for ClipboardContext {
    fn new() -> Result<Self> {
        Ok(Self {
            contents: Default::default(),
        })
    }
    fn get_contents(&mut self) -> Result<String> {
        Ok(self.contents.clone())
    }
    fn set_contents(&mut self, contents: String) -> Result<()> {
        self.contents = contents;
        Ok(())
    }
    fn clear(&mut self) -> Result<()> {
        self.contents = Default::default();
        Ok(())
    }
}
