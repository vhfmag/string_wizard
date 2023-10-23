use crate::{CowStr, MagicString};

fn guess_indent_str(source: &str) -> Option<String> {
    let mut tabbed_count = 0;
    let mut spaced_line = vec![];
    for line in source.lines() {
        if line.starts_with('\t') {
            tabbed_count += 1;
        } else if line.starts_with("  ") {
            spaced_line.push(line);
        }
    }

    if tabbed_count == 0 && spaced_line.is_empty() {
        return None;
    }

    if tabbed_count >= spaced_line.len() {
        return Some("\t".to_string());
    }

    let min_space_count = spaced_line
        .iter()
        .map(|line| line.chars().take_while(|c| *c == ' ').count())
        .min()
        .unwrap_or(0);

    let mut indent_str = String::with_capacity(min_space_count);
    for _ in 0..min_space_count {
        indent_str.push(' ');
    }
    Some(indent_str)
}

#[derive(Debug, Default)]
pub struct IndentOptions<'a> {
    /// MagicString will guess the `indent_str`` from lines of the source if passed `None`.
    pub indent_str: Option<&'a str>,
}

impl<'text> MagicString<'text> {
    fn ensure_indent_str(&mut self) -> &str {
        if self.indent_str.is_none() {
            self.indent_str = guess_indent_str(&self.source);
        }

        self.indent_str.as_deref().unwrap_or(&"\t")
    }

    pub fn indent(&mut self) -> &mut Self {
        self.indent_with(IndentOptions { indent_str: None })
    }

    /// Shortcut for `indent_with(IndentOptions { indent_str: Some(indent_str), ..Default::default() })`
    pub fn indent_str(&mut self, indent_str: &str) -> &mut Self {
        self.indent_with(IndentOptions {
            indent_str: Some(indent_str),
        })
    }

    pub fn indent_with(&mut self, opts: IndentOptions<'_>) -> &mut Self {
        if opts.indent_str.map_or(false, |s| s.is_empty()) {
            return self;
        }
        struct IndentReplacer {
            should_indent_next_char: bool,
            indent_str: String,
        }

        impl regex::Replacer for &mut &mut IndentReplacer {
            fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
                if self.should_indent_next_char {
                    dst.push_str(&self.indent_str);
                }
                for cap in caps.iter() {
                    if let Some(cap) = cap {
                        dst.push_str(cap.as_str());
                    }
                }
            }
        }

        fn indent_frag<'text>(
            frag: &mut CowStr<'text>,
            pattern: &regex::Regex,
            mut indent_replacer: &mut IndentReplacer,
        ) {
            match frag {
                std::borrow::Cow::Borrowed(str) => {
                    let might_replaced = pattern.replace_all(str, &mut indent_replacer);
                    *frag = might_replaced;
                }
                std::borrow::Cow::Owned(str) => {
                    let might_replaced = pattern.replace_all(str, &mut indent_replacer);
                    match might_replaced {
                        std::borrow::Cow::Owned(replaced) => {
                            *frag = replaced.into();
                        }
                        std::borrow::Cow::Borrowed(_) => {
                            // Since nothing was replaced, we can just use the original string.
                        }
                    }
                }
            }
        }

        let indent_str = opts.indent_str.unwrap_or_else(|| self.ensure_indent_str());

        let pattern = regex::Regex::new(r"(?m)^[^\r\n]").unwrap();

        let mut indent_replacer = IndentReplacer {
            should_indent_next_char: true,
            indent_str: indent_str.to_string(),
        };

        for intro_frag in self.intro.iter_mut() {
            indent_frag(intro_frag, &pattern, &mut indent_replacer)
        }

        let mut next_chunk_id = Some(self.first_chunk_idx);

        while let Some(chunk_idx) = next_chunk_id {
            // Make sure the `next_chunk_id` is updated before we split the chunk. Otherwise, we
            // might process the same chunk twice.
            next_chunk_id = self.chunks[chunk_idx].next;
            if let Some(edited_content) = self.chunks[chunk_idx].edited_content.as_mut() {
                indent_frag(edited_content, &pattern, &mut indent_replacer);
            } else {
                let chunk = &self.chunks[chunk_idx];
                let mut line_starts = vec![];
                let mut char_index = chunk.start();
                for char in chunk.span.text(&self.source).chars() {
                    debug_assert!(self.source.is_char_boundary(char_index as usize));
                    if char == '\n' {
                        indent_replacer.should_indent_next_char = true;
                    } else if char != '\r' && indent_replacer.should_indent_next_char {
                        indent_replacer.should_indent_next_char = false;
                        debug_assert!(!line_starts.contains(&char_index));
                        line_starts.push(char_index);
                    }
                    char_index += char.len_utf8() as u32;
                }
                for line_start in line_starts {
                    self.prepend_right(line_start, indent_replacer.indent_str.clone());
                }
            }
        }

        for frag in self.outro.iter_mut() {
            indent_frag(frag, &pattern, &mut indent_replacer)
        }

        self
    }
}