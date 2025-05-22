#[derive(Default)]
pub struct LatexBuilder {
    content: Vec<String>,
}

impl LatexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_command(&mut self, command: &str, args: &[&str]) -> &mut Self {
        let formatted_args = args
            .iter()
            .map(|x| format!("{{{}}}", x))
            .collect::<Vec<String>>()
            .join("");

        self.content
            .push(format!("\\{}{}", command, formatted_args));
        self
    }

    pub fn add_simple_command(&mut self, command: &str, arg: &str) -> &mut Self {
        self.add_command(command, &[arg])
    }

    pub fn add_env(&mut self, env: &str, content: &LatexBuilder) -> &mut Self {
        self.add_simple_command("begin", env);
        self.content.extend(content.content.iter().cloned());
        self.add_simple_command("end", env)
    }

    pub fn build(&self) -> String {
        self.content.join("\n")
    }
}
