use crate::{chunk::EditOptions, CowStr, MagicString, TextSize};

#[derive(Debug, Default, Clone)]
pub struct UpdateOptions {
    pub store_name: bool,
    pub overwrite: bool,
}

impl<'text> MagicString<'text> {
    pub fn update(
        &mut self,
        start: TextSize,
        end: TextSize,
        content: impl Into<CowStr<'text>>,
    ) -> &mut Self {
        self.update_with(
            start,
            end,
            content,
            UpdateOptions {
                store_name: false,
                overwrite: true,
            },
        )
    }

    pub fn update_with(
        &mut self,
        start: TextSize,
        end: TextSize,
        content: impl Into<CowStr<'text>>,
        opts: UpdateOptions,
    ) -> &mut Self {
        self.update_with_inner(start, end, content.into(), opts, true);
        self
    }

    pub fn remove(&mut self, start: TextSize, end: TextSize) -> &mut Self {
        self.update_with_inner(
            start,
            end,
            "".into(),
            UpdateOptions {
                store_name: false,
                overwrite: true,
            },
            false,
        );

        self
    }

    // --- private

    fn update_with_inner(
        &mut self,
        start: TextSize,
        end: TextSize,
        content: CowStr<'text>,
        opts: UpdateOptions,
        panic_if_start_equal_end: bool,
    ) -> &mut Self {
        if panic_if_start_equal_end && start == end {
            panic!(
                "Cannot overwrite a zero-length range – use append_left or prepend_right instead"
            )
        }
        assert!(start < end);
        self.split_at(start);
        self.split_at(end);

        let start_idx = self.chunk_by_start.get(&start).copied().unwrap();
        let end_idx = self.chunk_by_end.get(&end).copied().unwrap();

        let start_chunk = &mut self.chunks[start_idx];
        start_chunk.edit(
            content.into(),
            EditOptions {
                overwrite: opts.overwrite,
                store_name: opts.store_name,
            },
        );

        let mut rest_chunk_idx = if start_idx != end_idx {
            start_chunk.next.unwrap()
        } else {
            return self;
        };

        while rest_chunk_idx != end_idx {
            let rest_chunk = &mut self.chunks[rest_chunk_idx];
            rest_chunk.edit("".into(), Default::default());
            rest_chunk_idx = rest_chunk.next.unwrap();
        }
        self
    }
}