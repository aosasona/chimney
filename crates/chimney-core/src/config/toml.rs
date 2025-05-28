use super::Format;

pub struct TomlConfig<'a> {
    document: &'a str,
}

impl<'a> Format<'a> for TomlConfig<'a> {
    fn set_input(mut self, document: &'a str) {
        self.document = document
    }

    fn parse(self) -> super::Config {
        todo!()
    }
}
