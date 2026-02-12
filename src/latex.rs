#[derive(Default)]
pub struct LatexBuilder {
    content: Vec<String>,
}

pub struct Arg {
    value: String,
    optional: bool,
}

impl Arg {
    pub fn new(value: &str, optional: bool) -> Self {
        Self {
            value: value.to_string(),
            optional,
        }
    }

    pub fn optional(value: &str) -> Self {
        Self::new(value, true)
    }

    pub fn required(value: &str) -> Self {
        Self::new(value, false)
    }
}

impl LatexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_command(&mut self, command: &str, args: &[Arg]) -> &mut Self {
        let formatted_args: String = args
            .iter()
            .map(|arg| {
                if arg.optional {
                    format!("[{}]", arg.value)
                } else {
                    format!("{{{}}}", arg.value)
                }
            })
            .collect();

        self.content.push(format!("\\{command}{formatted_args}"));
        self
    }

    pub fn add_simple_command(&mut self, command: &str, arg: &str) -> &mut Self {
        self.add_command(command, &[Arg::required(arg)])
    }

    pub fn add_env(&mut self, env: &str, content: &LatexBuilder) -> &mut Self {
        self.add_simple_command("begin", env);
        self.add_builder(content);
        self.add_simple_command("end", env)
    }

    pub fn add_builder(&mut self, other: &LatexBuilder) -> &mut Self {
        self.content.extend(other.content.iter().cloned());
        self
    }

    pub fn build(&self) -> String {
        self.content.join("\n")
    }
}
