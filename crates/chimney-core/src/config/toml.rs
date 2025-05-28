use super::Format;

pub struct TomlConfig<'a> {
    document: &'a str,
}

impl<'a> Format<'a> for TomlConfig<'a> {
    fn set_input(mut self, input: &'a str) {
        self.document = input
    }

    fn parse(self) -> super::Config {
        todo!()
    }
}
