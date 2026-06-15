pub struct DartWriter {
    out: String,
    indent: usize,
}

impl DartWriter {
    pub fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
        }
    }

    pub fn into_string(self) -> String {
        self.out
    }

    pub fn indent_level(&self) -> usize {
        self.indent
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    pub fn blank(&mut self) {
        self.out.push('\n');
    }

    pub fn line(&mut self, content: &str) {
        self.write_indent();
        self.out.push_str(content);
        self.out.push('\n');
    }

    pub fn lines(&mut self, content: &str) {
        for line in content.lines() {
            self.line(line);
        }
    }

    pub fn block<F>(&mut self, opener: &str, closer: &str, body: F)
    where
        F: FnOnce(&mut Self),
    {
        self.line(opener);
        self.indent();
        body(self);
        self.dedent();
        self.line(closer);
    }

    pub fn try_block<F, E>(&mut self, opener: &str, closer: &str, body: F) -> Result<(), E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        self.line(opener);
        self.indent();
        let result = body(self);
        self.dedent();
        self.line(closer);
        result
    }

    pub fn documentation(&mut self, text: &str) {
        for line in text.lines() {
            let trimmed = line.trim_start().trim_end();
            for wrapped in wrap_comment_line(trimmed, self.indent * 2 + 4) {
                self.line(&format!("/// {wrapped}"));
            }
        }
    }

    pub fn fits(&self, content: &str) -> bool {
        self.indent * 2 + content.len() <= 80
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }
}

impl Default for DartWriter {
    fn default() -> Self {
        Self::new()
    }
}

fn wrap_comment_line(text: &str, prefix_len: usize) -> Vec<String> {
    let max_len = 80usize.saturating_sub(prefix_len);
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }

        if current.len() + 1 + word.len() <= max_len {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}
